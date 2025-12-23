use std::time::Duration;

use gtk::prelude::*;
use relm4::prelude::*;
pub struct Battery {
    info: String,
}

impl Battery {
    fn get_info() -> String {
        let manager = battery::Manager::new().ok();

        if let Some(mut batteries) = manager.and_then(|m| m.batteries().ok())
            && let Some(Ok(battery)) = batteries.next()
        {
            let state = battery.state();
            let charge = battery.state_of_charge().value * 100.0;

            let icon = match charge as u8 {
                90..=100 => "",
                60..=89 => "",
                40..=59 => "",
                10..=39 => "",
                _ => "",
            };

            let charging_icon = match state {
                battery::State::Charging => " ",
                _ => "",
            };

            return format!("{}{} {:.0}", charging_icon, icon, charge);
        }

        " ??%".to_string()
    }
}
#[derive(Debug)]
pub enum BatteryMsg {
    Update(String),
}

#[relm4::component(pub)]
impl Component for Battery {
    type Input = BatteryMsg;
    type Output = ();
    type Init = ();
    type CommandOutput = BatteryMsg;
    view! {
        #[root]
        gtk::Label {
            add_css_class: "battery-label",
            #[watch]
            set_label: &model.info,
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        sender.command(|out, _shutdown| async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;

                let info = tokio::task::spawn_blocking(Battery::get_info)
                    .await
                    .unwrap_or_else(|_| "Error".to_string());

                if out.send(BatteryMsg::Update(info)).is_err() {
                    break;
                }
            }
        });
        let model = Battery {
            info: Self::get_info(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            BatteryMsg::Update(new_info) => self.info = new_info,
        }
    }
    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.update(message, sender, root);
    }
}
