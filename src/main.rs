use eigen::ipc;
use eigen::widgets::{
    battery::Battery, clock::Clock, launcher::Launcher, launcher::LauncherMsg,
    workspaces::WorkspacesModel, dashboard::Dashboard, dashboard::DashboardMsg,
    desktop_menu::DesktopMenu,
};
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use relm4::prelude::*;

// ── Bar ──

struct BarConfig {
    title: String,
    timeformat: String,
    ws_count: u8,
}

struct Bar {
    workspaces: Controller<WorkspacesModel>,
    battery: Controller<Battery>,
    clock: Controller<Clock>,
    #[allow(dead_code)]
    launcher: Controller<Launcher>,
    #[allow(dead_code)]
    dashboard: Controller<Dashboard>,
    #[allow(dead_code)]
    desktop_menu: Controller<DesktopMenu>,
}

#[relm4::component]
impl SimpleComponent for Bar {
    type Init = BarConfig;
    type Input = BarMsg;
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
                set_start_widget = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,
                    gtk::Button {
                        set_label: "",
                        add_css_class: "dashboard-btn",
                        connect_clicked[sender] => move |_| {
                            sender.input(BarMsg::ToggleDashboard);
                        }
                    },
                    append = model.workspaces.widget(),
                },
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
                },
            }
        }
    }

    fn init(
        config: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.init_layer_shell();
        root.set_layer(Layer::Top);
        root.set_anchor(Edge::Bottom, true);
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
        let launcher = Launcher::builder().launch(()).detach();
        let dashboard = Dashboard::builder().launch(()).detach();
        let desktop_menu = DesktopMenu::builder().launch(()).detach();

        // ── Start IPC listener ──
        let launcher_sender = launcher.sender().clone();
        let dashboard_sender = dashboard.sender().clone();
        ipc::start_listener(move |cmd| {
            match cmd.trim() {
                "toggle-launcher" => {
                    // Send the toggle message to the launcher component
                    launcher_sender.send(LauncherMsg::Toggle).ok();
                }
                "toggle-dashboard" => {
                    dashboard_sender.send(DashboardMsg::Toggle).ok();
                }
                other => {
                    eprintln!("[eigen] Unknown IPC command: {other}");
                }
            }
        });

        let model = Bar {
            workspaces,
            clock,
            battery,
            launcher,
            dashboard,
            desktop_menu,
        };

        let widgets = view_output!();
        root.present();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            BarMsg::ToggleDashboard => {
                self.dashboard.sender().send(DashboardMsg::Toggle).ok();
            }
        }
    }
}

#[derive(Debug)]
enum BarMsg {
    ToggleDashboard,
}

const CSS_STR: &str = include_str!("css/style.css");

use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize the daemon
    Init,
    /// Toggle the launcher
    ToggleLauncher,
    /// Toggle the dashboard
    ToggleDashboard,
    /// View logs
    Logs,
}

fn main() {
    let cli = Cli::parse();
    
    let log_file_path = "/tmp/eigen.log";

    if let Some(Commands::Logs) = cli.command {
        let _ = std::process::Command::new("tail")
            .arg("-f")
            .arg(log_file_path)
            .status();
        return;
    }

    let file_appender = tracing_appender::rolling::never("/tmp", "eigen.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .init();

    tracing::info!("Starting eigen shell...");

    match cli.command.unwrap_or(Commands::Init) {
        Commands::Init => {
            // Run the daemon (bar + launcher)
            let app = RelmApp::new("com.eigen.shell");
            app.run::<Bar>(BarConfig {
                title: "Eigen Bar".into(),
                timeformat: "%H:%M:%S".into(),
                ws_count: 5,
            });
        }
        Commands::ToggleLauncher => {
            // Send IPC signal to the running daemon
            if let Err(e) = ipc::send_command("toggle-launcher") {
                tracing::error!("Failed to reach daemon: {e}");
                eprintln!("eigen: Failed to reach daemon: {e}");
                eprintln!("  Is `eigen init` running?");
                std::process::exit(1);
            }
        }
        Commands::ToggleDashboard => {
            if let Err(e) = ipc::send_command("toggle-dashboard") {
                tracing::error!("Failed to reach daemon: {e}");
                eprintln!("eigen: Failed to reach daemon: {e}");
                eprintln!("  Is `eigen init` running?");
                std::process::exit(1);
            }
        }
        Commands::Logs => unreachable!(),
    }
}
