pub mod desktop;
pub mod trie;

use crate::widgets::launcher::trie::Trie;

use desktop::DesktopEntry;
use gtk::gdk;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use relm4::prelude::*;

// ── Messages ──

#[derive(Debug)]
pub enum LauncherMsg {
    Toggle,
    SearchChanged(String),
    Activate(usize),
    ClickedOutside,
    Hide,
}

// ── Model ──

pub struct Launcher {
    all_entries: Vec<DesktopEntry>,
    filtered: Vec<DesktopEntry>,
    query: String,
    visible: bool,
    trie: Trie,
}

impl Launcher {
    fn apply_filter(&mut self) {
        let q = self.query.trim().to_lowercase();
        if q.is_empty() {
            self.filtered = self.all_entries.clone();
        } else {
            let mut unique_indices = self.trie.search(&q);
            unique_indices.sort_unstable();
            unique_indices.dedup();
            self.filtered = unique_indices
                .into_iter()
                .filter_map(|idx| self.all_entries.get(idx))
                .cloned()
                .collect();
        }
    }

    fn build_trie(&mut self) {
        self.trie = Trie::new();
        for (i, entry) in self.all_entries.iter().enumerate() {
            self.trie.insert(&entry.name, i);
            if !entry.description.is_empty() {
                self.trie.insert(&entry.description, i);
            }
        }
        tracing::debug!("Built desktop entries Trie with {} apps", self.all_entries.len());
    }

    fn launch_app(&self, index: usize) {
        if let Some(entry) = self.filtered.get(index) {
            tracing::info!("Launching application: {}", entry.name);
            let exec = entry.exec.clone();
            std::thread::spawn(move || {
                let parts: Vec<&str> = exec.split_whitespace().collect();
                if let Some((cmd, args)) = parts.split_first() {
                    let _ = std::process::Command::new(cmd)
                        .args(args)
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();
                }
            });
        }
    }
}

// ── Component ──

#[relm4::component(pub)]
impl Component for Launcher {
    type Init = ();
    type Input = LauncherMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Window {
            add_css_class: "launcher-window",
            set_decorated: false,

            #[name = "revealer"]
            gtk::Revealer {
                set_transition_type: gtk::RevealerTransitionType::SlideUp,
                set_transition_duration: 150,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_valign: gtk::Align::End,

                    gtk::Box {
                        add_css_class: "inverse-corner-left",
                        set_valign: gtk::Align::End,
                    },

                    // ── Main Panel ──
                    gtk::Box {
                        add_css_class: "launcher-panel",
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::Box {
                            add_css_class: "launcher-panel-inner",
                            set_orientation: gtk::Orientation::Vertical,

                            #[name = "search_entry"]
                            gtk::SearchEntry {
                                add_css_class: "launcher-search",
                                set_placeholder_text: Some("Search applications…"),
                                set_hexpand: true,
                                connect_search_changed[sender] => move |entry| {
                                    sender.input(LauncherMsg::SearchChanged(entry.text().to_string()));
                                },
                                connect_activate[sender, list_box] => move |_| {
                                    if let Some(row) = list_box.selected_row() {
                                        sender.input(LauncherMsg::Activate(row.index() as usize));
                                    }
                                }
                            },

                            #[name = "scroll"]
                            gtk::ScrolledWindow {
                                add_css_class: "launcher-scroll",
                                set_vexpand: true,
                                set_hscrollbar_policy: gtk::PolicyType::Never,
                                set_min_content_height: 400,

                                #[name = "list_box"]
                                gtk::ListBox {
                                    add_css_class: "launcher-list",
                                    set_selection_mode: gtk::SelectionMode::Browse,
                                    set_activate_on_single_click: true,
                                },
                            },
                        },
                    },

                    gtk::Box {
                        add_css_class: "inverse-corner-right",
                        set_valign: gtk::Align::End,
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // ── Layer Shell Setup ──
        root.init_layer_shell();
        root.set_layer(Layer::Top);
        root.set_namespace(Some("eigen-launcher"));

        // Anchor only to the bottom. The window manager will position it just above the bar.
        root.set_anchor(Edge::Bottom, true);

        root.set_keyboard_mode(KeyboardMode::OnDemand);

        // Load launcher CSS
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // ── Load desktop entries ──
        tracing::info!("Loading desktop entries...");
        let all_entries = desktop::load_desktop_entries();

        let mut model = Launcher {
            filtered: all_entries.clone(),
            all_entries,
            query: String::new(),
            visible: false,
            trie: Trie::new(),
        };
        model.build_trie();

        let widgets = view_output!();

        // ── Populate the list ──
        populate_list(&widgets.list_box, &model.filtered);

        // ── Connect ListBox row-activated ──
        let s = sender.clone();
        widgets.list_box.connect_row_activated(move |_, row| {
            s.input(LauncherMsg::Activate(row.index() as usize));
        });



        // ── Focus-out handler for click-outside ──
        let s = sender.clone();
        root.connect_is_active_notify(move |win| {
            if !win.is_active() {
                s.input(LauncherMsg::ClickedOutside);
            }
        });

        // ── Key handler for Enter, Escape, and Ctrl+{n,p,j,k} ──
        let key_ctl = gtk::EventControllerKey::new();
        let s = sender.clone();
        let lb = widgets.list_box.clone();
        key_ctl.connect_key_pressed(move |_, key, _, modifiers| {
            let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);

            match key {
                gdk::Key::Escape => {
                    s.input(LauncherMsg::Hide);
                    gtk::glib::Propagation::Stop
                }
                gdk::Key::Return | gdk::Key::KP_Enter => {
                    if let Some(row) = lb.selected_row() {
                        s.input(LauncherMsg::Activate(row.index() as usize));
                    }
                    gtk::glib::Propagation::Stop
                }
                // Ctrl+N / Ctrl+J → select next row
                gdk::Key::n | gdk::Key::j if ctrl => {
                    move_selection(&lb, 1);
                    gtk::glib::Propagation::Stop
                }
                // Ctrl+P / Ctrl+K → select previous row
                gdk::Key::p | gdk::Key::k if ctrl => {
                    move_selection(&lb, -1);
                    gtk::glib::Propagation::Stop
                }
                _ => gtk::glib::Propagation::Proceed,
            }
        });
        root.add_controller(key_ctl);

        // Start hidden
        root.set_visible(false);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            LauncherMsg::Toggle => {
                if self.visible {
                    hide_launcher(self, root);
                } else {
                    self.visible = true;
                    tracing::info!("Opening launcher");
                    self.apply_filter();
                    if let Some(lb_widget) =
                        find_child_by_css_class(root.child().as_ref(), "launcher-list")
                    {
                        if let Some(lb) = lb_widget.downcast_ref::<gtk::ListBox>() {
                            populate_list(lb, &self.filtered);
                            if let Some(first) = lb.row_at_index(0) {
                                lb.select_row(Some(&first));
                            }
                        }
                    }
                    root.set_visible(true);
                    root.present();
                    if let Some(revealer) =
                        find_child_by_type::<gtk::Revealer>(root.child().as_ref())
                    {
                        revealer.set_reveal_child(true);
                    }
                    if let Some(search) =
                        find_child_by_css_class(root.child().as_ref(), "launcher-search")
                    {
                        search.grab_focus();
                    }
                }
            }
            LauncherMsg::SearchChanged(q) => {
                self.query = q;
                if !self.visible {
                    return;
                }
                tracing::debug!("Search query: {}", self.query);
                self.apply_filter();
                if let Some(lb_widget) =
                    find_child_by_css_class(root.child().as_ref(), "launcher-list")
                {
                    if let Some(lb) = lb_widget.downcast_ref::<gtk::ListBox>() {
                        populate_list(lb, &self.filtered);
                        if let Some(first) = lb.row_at_index(0) {
                            lb.select_row(Some(&first));
                        }
                    }
                }
            }
            LauncherMsg::Activate(index) => {
                tracing::info!("Activating launcher item at index {}", index);
                self.launch_app(index);
                hide_launcher(self, root);
            }
            LauncherMsg::ClickedOutside => {
                if self.visible {
                    hide_launcher(self, root);
                }
            }
            LauncherMsg::Hide => {
                if self.visible {
                    tracing::info!("Hiding launcher via Escape key");
                    hide_launcher(self, root);
                }
            }
        }
    }
}

/// Instantly hide the launcher window with zero lag.
/// We skip the revealer close animation entirely — just hide the window,
/// then reset the revealer state while invisible so it can slide-up again next open.
fn hide_launcher(model: &mut Launcher, root: &gtk::Window) {
    model.visible = false;

    // 1. Hide the window FIRST — everything below happens off-screen
    root.set_visible(false);

    // 2. Reset the revealer while invisible (no animation rendered)
    if let Some(revealer) = find_child_by_type::<gtk::Revealer>(root.child().as_ref()) {
        revealer.set_transition_duration(0);
        revealer.set_reveal_child(false);
        revealer.set_transition_duration(150); // Restore for next open
    }

    // 3. Reset search entry (triggers SearchChanged but we bail early since !visible)
    if let Some(search) = find_child_by_css_class(root.child().as_ref(), "launcher-search") {
        if let Some(se) = search.downcast_ref::<gtk::SearchEntry>() {
            se.set_text("");
        }
    }
    model.query.clear();
}

/// Move the selected row in the ListBox by `delta` (1 = next, -1 = previous).
fn move_selection(list_box: &gtk::ListBox, delta: i32) {
    let current_idx = list_box
        .selected_row()
        .map(|r| r.index())
        .unwrap_or(-1);
    let next_idx = current_idx + delta;
    if next_idx >= 0 {
        if let Some(row) = list_box.row_at_index(next_idx) {
            list_box.select_row(Some(&row));
            // Scroll the selected row into view
            row.grab_focus();
        }
    }
}

/// Find a child widget of a specific GObject type by walking the tree.
fn find_child_by_type<T: gdk::prelude::IsA<gtk::Widget>>(
    root: Option<&gtk::Widget>,
) -> Option<T> {
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

/// Find a child widget by CSS class.
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

/// Populate the ListBox with desktop entries.
fn populate_list(list_box: &gtk::ListBox, entries: &[DesktopEntry]) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let icon_theme = gtk::IconTheme::for_display(&gdk::Display::default().unwrap());

    for entry in entries {
        let row_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .css_classes(["launcher-app-row"])
            .build();

        // Icon
        let image = if !entry.icon.is_empty() {
            if icon_theme.has_icon(&entry.icon) {
                gtk::Image::builder()
                    .icon_name(&entry.icon)
                    .pixel_size(40)
                    .css_classes(["launcher-app-icon"])
                    .build()
            } else if std::path::Path::new(&entry.icon).exists() {
                gtk::Image::builder()
                    .file(&entry.icon)
                    .pixel_size(40)
                    .css_classes(["launcher-app-icon"])
                    .build()
            } else {
                gtk::Image::builder()
                    .icon_name("application-x-executable")
                    .pixel_size(40)
                    .css_classes(["launcher-app-icon"])
                    .build()
            }
        } else {
            gtk::Image::builder()
                .icon_name("application-x-executable")
                .pixel_size(40)
                .css_classes(["launcher-app-icon"])
                .build()
        };

        row_box.append(&image);

        // Text column
        let text_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .build();

        let name_label = gtk::Label::builder()
            .label(&entry.name)
            .xalign(0.0)
            .css_classes(["launcher-app-name"])
            .build();
        text_box.append(&name_label);

        if !entry.description.is_empty() {
            let desc_label = gtk::Label::builder()
                .label(&entry.description)
                .xalign(0.0)
                .css_classes(["launcher-app-desc"])
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .max_width_chars(60)
                .build();
            text_box.append(&desc_label);
        }

        row_box.append(&text_box);
        list_box.append(&row_box);
    }
}
