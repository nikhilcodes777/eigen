use eigen::widgets::{battery::Battery, clock::Clock, workspaces::WorkspacesModel};
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use relm4::prelude::*;
struct BarConfig {
    title: String,
    timeformat: String,
    ws_count: u8,
}
struct Bar {
    workspaces: Controller<WorkspacesModel>,
    battery: Controller<Battery>,
    clock: Controller<Clock>,
}

#[relm4::component]
impl SimpleComponent for Bar {
    type Init = BarConfig;
    type Input = ();
    type Output = ();

    view! {
        #[root]
        gtk::Window {
            set_title: Some(config.title.as_str()),
            set_default_size: (1920, 24),
            set_decorated: false,

            gtk::CenterBox {
                set_margin_all: 5,
                #[wrap(Some)]
                set_start_widget = model.workspaces.widget(),
                #[wrap(Some)]
                set_center_widget = &gtk::Label {
                    set_label: config.title.as_str(),
                    add_css_class: "title-label",
                },

                #[wrap(Some)]
                set_end_widget = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,
                    append = model.battery.widget(),
                    append = model.clock.widget(),
                }
            }
        }
    }

    fn init(
        config: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.init_layer_shell();
        root.set_layer(Layer::Top);
        root.set_anchor(Edge::Top, true);
        root.set_anchor(Edge::Left, true);
        root.set_anchor(Edge::Right, true);
        root.auto_exclusive_zone_enable();

        let provider = gtk::CssProvider::new();
        provider.load_from_string(CSS_STR);
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let clock = Clock::builder().launch(config.timeformat).detach();
        let battery = Battery::builder().launch(()).detach();
        let workspaces = WorkspacesModel::builder().launch(config.ws_count).detach();
        let model = Bar {
            workspaces,
            clock,
            battery,
        };

        let widgets = view_output!();
        root.present();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _msg: Self::Input, _sender: ComponentSender<Self>) {}
}

const CSS_STR: &str = include_str!("css/style.css");
fn main() {
    let app = RelmApp::new("com.eigen.shell");
    app.run::<Bar>(BarConfig {
        title: "Eigen Bar".into(),
        timeformat: "%H:%M:%S".into(),
        ws_count: 5,
    });
}
