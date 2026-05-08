use crate::widgets::launcher::desktop::{self, DesktopEntry};
use gtk::gio;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use relm4::prelude::*;
use std::collections::HashMap;

#[derive(Debug)]
pub enum DesktopMenuMsg {
    ShowMenu(f64, f64),
    HideMenu,
}

pub struct DesktopMenu {
    #[allow(dead_code)]
    entries: Vec<DesktopEntry>,
    #[allow(dead_code)]
    menu_model: gio::Menu,
    popover: gtk::PopoverMenu,
}

#[relm4::component(pub)]
impl Component for DesktopMenu {
    type Init = ();
    type Input = DesktopMenuMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Window {
            add_css_class: "desktop-bg-window",
            set_decorated: false,

            #[name = "fixed"]
            gtk::Box {
                set_hexpand: true,
                set_vexpand: true,
                add_css_class: "desktop-menu-overlay",
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.init_layer_shell();
        root.set_layer(Layer::Bottom);
        root.set_namespace(Some("eigen-desktop-menu"));

        // Anchor to all edges to cover the entire background
        root.set_anchor(Edge::Top, true);
        root.set_anchor(Edge::Bottom, true);
        root.set_anchor(Edge::Left, true);
        root.set_anchor(Edge::Right, true);

        // Styling for the transparent background and popover menu
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let entries = desktop::load_desktop_entries();
        let menu_model = build_menu_model(&entries);

        let popover = gtk::PopoverMenu::from_model(Some(&menu_model));
        popover.set_has_arrow(false);
        popover.set_position(gtk::PositionType::Right);
        popover.add_css_class("desktop-popover-menu");

        let widgets = view_output!();
        
        popover.set_parent(&widgets.fixed);

        // Handle Right Click
        let click = gtk::GestureClick::new();
        click.set_button(gtk::gdk::BUTTON_SECONDARY);
        
        let s = sender.clone();
        click.connect_pressed(move |_, _, x, y| {
            s.input(DesktopMenuMsg::ShowMenu(x, y));
        });
        widgets.fixed.add_controller(click);

        // Handle Left Click to hide
        let left_click = gtk::GestureClick::new();
        left_click.set_button(gtk::gdk::BUTTON_PRIMARY);
        let s = sender.clone();
        left_click.connect_pressed(move |_, _, _, _| {
            s.input(DesktopMenuMsg::HideMenu);
        });
        widgets.fixed.add_controller(left_click);

        // Handle App Execution Actions
        let action_group = gio::SimpleActionGroup::new();
        for entry in &entries {
            let exec_cmd = entry.exec.clone();
            let action_name = sanitize_action_name(&entry.name);
            let action = gio::SimpleAction::new(&action_name, None);
            
            action.connect_activate(move |_, _| {
                let exec = exec_cmd.clone();
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
            });
            action_group.add_action(&action);
        }
        
        root.insert_action_group("app", Some(&action_group));

        let model = DesktopMenu {
            entries,
            menu_model,
            popover,
        };

        root.present();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DesktopMenuMsg::ShowMenu(x, y) => {
                self.popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                self.popover.popup();
            }
            DesktopMenuMsg::HideMenu => {
                self.popover.popdown();
            }
        }
    }
}

fn sanitize_action_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

fn build_menu_model(entries: &[DesktopEntry]) -> gio::Menu {
    let root_menu = gio::Menu::new();
    
    let mut categorized: HashMap<String, Vec<&DesktopEntry>> = HashMap::new();
    
    // Core categories typically found in .desktop files
    let core_categories = ["Utility", "Development", "Game", "Graphics", "Network", "AudioVideo", "Office", "System", "Settings"];
    
    for entry in entries {
        let mut assigned = false;
        for cat in &entry.categories {
            if core_categories.contains(&cat.as_str()) {
                categorized.entry(cat.to_string()).or_default().push(entry);
                assigned = true;
                break;
            }
        }
        if !assigned {
            categorized.entry("Other".to_string()).or_default().push(entry);
        }
    }
    
    let mut sorted_categories: Vec<_> = categorized.keys().cloned().collect();
    sorted_categories.sort();
    
    for cat in sorted_categories {
        let apps = categorized.get(&cat).unwrap();
        let sub_menu = gio::Menu::new();
        
        for app in apps {
            let action_name = format!("app.{}", sanitize_action_name(&app.name));
            let item = gio::MenuItem::new(Some(&app.name), Some(&action_name));
            sub_menu.append_item(&item);
        }
        
        root_menu.append_submenu(Some(&cat), &sub_menu);
    }
    
    root_menu
}
