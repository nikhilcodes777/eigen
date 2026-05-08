use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use relm4::prelude::*;
use std::collections::VecDeque;
use std::time::Duration;
use sysinfo::{LoadAvg, Networks, System};

#[derive(Debug)]
pub enum DashboardMsg {
    Toggle,
    Hide,
    Tick,
}

pub struct Dashboard {
    visible: bool,
    system: System,
    networks: Networks,
    
    // Historical data for charts (last 60 seconds)
    cpu_usage: f64,
    ram_used: u64,
    ram_total: u64,
    swap_used: u64,
    swap_total: u64,
    uptime: u64,
    load_avg: LoadAvg,
    net_rx: u64,
    net_tx: u64,

    history_shared: std::sync::Arc<std::sync::Mutex<GraphHistory>>,
}

pub struct GraphHistory {
    cpu: VecDeque<f64>,
    ram: VecDeque<f64>,
    swap: VecDeque<f64>,
}

#[relm4::component(pub)]
impl Component for Dashboard {
    type Init = ();
    type Input = DashboardMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Window {
            add_css_class: "dashboard-window",
            set_decorated: false,

            #[name = "revealer"]
            gtk::Revealer {
                set_transition_type: gtk::RevealerTransitionType::SlideUp,
                set_transition_duration: 250,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_valign: gtk::Align::End,
                    
                    gtk::Box {
                        add_css_class: "dashboard-panel",
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 15,
                        
                        // User Profile Section
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 15,
                            add_css_class: "profile-section",
                            
                            gtk::Image {
                                set_icon_name: Some("avatar-default"),
                                set_pixel_size: 64,
                                add_css_class: "profile-pic",
                            },
                            
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_valign: gtk::Align::Center,
                                gtk::Label {
                                    set_label: &std::env::var("USER").unwrap_or_else(|_| "User".into()),
                                    add_css_class: "profile-name",
                                    set_halign: gtk::Align::Start,
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &format!("Uptime: {}h {}m", model.uptime / 3600, (model.uptime % 3600) / 60),
                                    add_css_class: "profile-uptime",
                                    set_halign: gtk::Align::Start,
                                },
                            }
                        },
                        
                        // System Stats Section
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 10,
                            add_css_class: "stats-section",
                            
                            gtk::Label {
                                set_label: "System Status",
                                add_css_class: "section-title",
                                set_halign: gtk::Align::Start,
                            },
                            
                            // CPU
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 10,
                                gtk::Label { set_label: "CPU", add_css_class: "stat-label", set_width_chars: 5, set_halign: gtk::Align::Start },
                                gtk::ProgressBar {
                                    #[watch]
                                    set_fraction: model.cpu_usage / 100.0,
                                    set_hexpand: true,
                                    add_css_class: "cpu-bar",
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &format!("{:>4.1}%", model.cpu_usage),
                                    add_css_class: "stat-value",
                                }
                            },
                            
                            // RAM
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 10,
                                gtk::Label { set_label: "RAM", add_css_class: "stat-label", set_width_chars: 5, set_halign: gtk::Align::Start },
                                gtk::ProgressBar {
                                    #[watch]
                                    set_fraction: if model.ram_total > 0 { model.ram_used as f64 / model.ram_total as f64 } else { 0.0 },
                                    set_hexpand: true,
                                    add_css_class: "ram-bar",
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &format!("{:>4.1}GB", model.ram_used as f64 / 1073741824.0),
                                    add_css_class: "stat-value",
                                }
                            },
                            
                            // Swap
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 10,
                                gtk::Label { set_label: "Swap", add_css_class: "stat-label", set_width_chars: 5, set_halign: gtk::Align::Start },
                                gtk::ProgressBar {
                                    #[watch]
                                    set_fraction: if model.swap_total > 0 { model.swap_used as f64 / model.swap_total as f64 } else { 0.0 },
                                    set_hexpand: true,
                                    add_css_class: "swap-bar",
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &format!("{:>4.1}GB", model.swap_used as f64 / 1073741824.0),
                                    add_css_class: "stat-value",
                                }
                            },
                        },
                        
                        // Graphs Section
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 5,
                            add_css_class: "graphs-section",
                            
                            #[name = "cpu_graph"]
                            gtk::DrawingArea {
                                set_height_request: 120,
                                set_hexpand: true,
                                add_css_class: "graph-area",
                            },
                        },
                        
                        // Network & Load
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 15,
                            set_homogeneous: true,
                            
                            // Load Average
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                add_css_class: "small-stat-box",
                                gtk::Label { set_label: "Load Average", add_css_class: "small-stat-label" },
                                gtk::Label {
                                    #[watch]
                                    set_label: &format!("{:.2} {:.2} {:.2}", model.load_avg.one, model.load_avg.five, model.load_avg.fifteen),
                                    add_css_class: "small-stat-value",
                                }
                            },
                            
                            // Network
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                add_css_class: "small-stat-box",
                                gtk::Label { set_label: "Network (Rx/Tx)", add_css_class: "small-stat-label" },
                                gtk::Label {
                                    #[watch]
                                    set_label: &format!("{:.1} / {:.1} KB/s", model.net_rx as f64 / 1024.0, model.net_tx as f64 / 1024.0),
                                    add_css_class: "small-stat-value",
                                }
                            }
                        }
                    },
                    
                    gtk::Box {
                        add_css_class: "inverse-corner-right",
                        set_valign: gtk::Align::End,
                    },
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.init_layer_shell();
        root.set_layer(Layer::Top);
        root.set_namespace(Some("eigen-dashboard"));

        // Anchor bottom-left
        root.set_anchor(Edge::Bottom, true);
        root.set_anchor(Edge::Left, true);
        root.set_margin(Edge::Left, 10);
        
        // Don't take focus aggressively
        root.set_keyboard_mode(KeyboardMode::OnDemand);

        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let mut system = System::new_all();
        system.refresh_cpu_usage();
        
        let history = GraphHistory {
            cpu: vec![0.0; 60].into(),
            ram: vec![0.0; 60].into(),
            swap: vec![0.0; 60].into(),
        };
        let history_shared = std::sync::Arc::new(std::sync::Mutex::new(history));
        
        let model = Dashboard {
            visible: false,
            system,
            networks: Networks::new_with_refreshed_list(),
            history_shared: history_shared.clone(),
            cpu_usage: 0.0,
            ram_used: 0,
            ram_total: 1,
            swap_used: 0,
            swap_total: 1,
            uptime: 0,
            load_avg: LoadAvg { one: 0.0, five: 0.0, fifteen: 0.0 },
            net_rx: 0,
            net_tx: 0,
        };

        let widgets = view_output!();
        
        // Custom draw function for multi-graph
        let history_for_draw = history_shared.clone();
        
        widgets.cpu_graph.set_draw_func(move |_, cr, width, height| {
            let width = width as f64;
            let height = height as f64;
            
            // Background
            cr.set_source_rgba(0.1, 0.1, 0.1, 0.5);
            cr.paint().unwrap();
            
            let history = history_for_draw.lock().unwrap();
            if history.cpu.is_empty() { return; }
            
            let step = width / (history.cpu.len() - 1) as f64;
            
            // Helper to draw a single line
            let draw_line = |data: &VecDeque<f64>, r: f64, g: f64, b: f64| {
                cr.set_line_width(2.0);
                cr.move_to(0.0, height);
                for (i, &val) in data.iter().enumerate() {
                    let x = i as f64 * step;
                    let y = height - (val / 100.0 * height);
                    cr.line_to(x, y);
                }
                cr.set_source_rgba(r, g, b, 1.0);
                cr.stroke().unwrap();
            };
            
            // Swap: Yellow/Peach (#f9e2af)
            draw_line(&history.swap, 0.976, 0.886, 0.686);
            // RAM: Green (#a6e3a1)
            draw_line(&history.ram, 0.651, 0.890, 0.631);
            // CPU: Blue (#89b4fa)
            draw_line(&history.cpu, 0.537, 0.706, 0.980);
            
            // Draw Legend at Top-Right
            let legend_x = width - 70.0;
            let mut legend_y = 10.0;
            
            let mut draw_legend_item = |label: &str, r: f64, g: f64, b: f64| {
                cr.set_source_rgba(r, g, b, 1.0);
                cr.rectangle(legend_x, legend_y, 10.0, 10.0);
                cr.fill().unwrap();
                
                cr.set_source_rgba(0.8, 0.8, 0.8, 1.0);
                cr.move_to(legend_x + 15.0, legend_y + 9.0);
                cr.set_font_size(10.0);
                cr.show_text(label).unwrap();
                
                legend_y += 15.0;
            };
            
            draw_legend_item("CPU", 0.537, 0.706, 0.980);
            draw_legend_item("RAM", 0.651, 0.890, 0.631);
            draw_legend_item("Swap", 0.976, 0.886, 0.686);
        });

        // ── Focus-out handler ──
        let s = sender.clone();
        root.connect_is_active_notify(move |win| {
            if !win.is_active() {
                s.input(DashboardMsg::Hide);
            }
        });

        // Tick timer for updating stats
        let s = sender.clone();
        gtk::glib::timeout_add_local(Duration::from_secs(1), move || {
            s.input(DashboardMsg::Tick);
            gtk::glib::ControlFlow::Continue
        });

        root.set_visible(false);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            DashboardMsg::Toggle => {
                if self.visible {
                    self.visible = false;
                    root.set_visible(false);
                    if let Some(revealer) = find_child_by_type::<gtk::Revealer>(root.child().as_ref()) {
                        revealer.set_transition_duration(0);
                        revealer.set_reveal_child(false);
                        revealer.set_transition_duration(250);
                    }
                } else {
                    self.visible = true;
                    root.set_visible(true);
                    root.present();
                    if let Some(revealer) = find_child_by_type::<gtk::Revealer>(root.child().as_ref()) {
                        revealer.set_reveal_child(true);
                    }
                }
            }
            DashboardMsg::Hide => {
                if self.visible {
                    self.visible = false;
                    root.set_visible(false);
                    if let Some(revealer) = find_child_by_type::<gtk::Revealer>(root.child().as_ref()) {
                        revealer.set_transition_duration(0);
                        revealer.set_reveal_child(false);
                        revealer.set_transition_duration(250);
                    }
                }
            }
            DashboardMsg::Tick => {
                // Update sysinfo
                self.system.refresh_cpu_usage();
                self.system.refresh_memory();
                self.networks.refresh(true);
                
                self.cpu_usage = self.system.global_cpu_usage() as f64;
                self.ram_used = self.system.used_memory();
                self.ram_total = self.system.total_memory();
                self.swap_used = self.system.used_swap();
                self.swap_total = self.system.total_swap();
                self.uptime = System::uptime();
                self.load_avg = System::load_average();
                
                let mut rx = 0;
                let mut tx = 0;
                for (_, data) in &self.networks {
                    rx += data.received();
                    tx += data.transmitted();
                }
                self.net_rx = rx;
                self.net_tx = tx;
                
                // Update history
                {
                    let mut history = self.history_shared.lock().unwrap();
                    history.cpu.pop_front();
                    history.cpu.push_back(self.cpu_usage);
                    
                    history.ram.pop_front();
                    let ram_percent = if self.ram_total > 0 { (self.ram_used as f64 / self.ram_total as f64) * 100.0 } else { 0.0 };
                    history.ram.push_back(ram_percent);
                    
                    history.swap.pop_front();
                    let swap_percent = if self.swap_total > 0 { (self.swap_used as f64 / self.swap_total as f64) * 100.0 } else { 0.0 };
                    history.swap.push_back(swap_percent);
                }
                
                // Trigger redraw of graph
                if let Some(drawing_area) = find_child_by_css_class(root.child().as_ref(), "graph-area") {
                    drawing_area.queue_draw();
                }
            }
        }
    }
}

// Helper to find child by type
fn find_child_by_type<T: gtk::prelude::IsA<gtk::Widget>>(root: Option<&gtk::Widget>) -> Option<T> {
    let root = root?;
    if let Some(found) = root.downcast_ref::<T>() {
        return Some(found.clone());
    }
    let mut child = root.first_child();
    while let Some(c) = child {
        if let Some(found) = find_child_by_type::<T>(Some(&c)) {
            return Some(found);
        }
        child = c.next_sibling();
    }
    None
}

// Helper to find child by css class
fn find_child_by_css_class(root: Option<&gtk::Widget>, class: &str) -> Option<gtk::Widget> {
    let root = root?;
    if root.has_css_class(class) {
        return Some(root.clone());
    }
    let mut child = root.first_child();
    while let Some(c) = child {
        if let Some(found) = find_child_by_css_class(Some(&c), class) {
            return Some(found);
        }
        child = c.next_sibling();
    }
    None
}
