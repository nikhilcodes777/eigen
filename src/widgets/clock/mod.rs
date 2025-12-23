use std::time::Duration;

use chrono::Local;
use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub enum ClockMsg {
    Tick,
}

#[derive(Debug)]
pub struct Clock {
    time: String,
    format: String,
}

#[relm4::component(pub)]
impl Component for Clock {
    type CommandOutput = ClockMsg;
    type Input = ClockMsg;
    type Output = ();
    type Init = String;

    view! {
        #[root]
        gtk::Label {
            add_css_class: "clock-label",
            #[watch]
            set_label: &model.time,
        }
    }

    fn init(
        init: Self::Init,
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
        let model = Clock {
            time: Local::now().format(&init).to_string(),
            format: init,
        };
        sender.command(|out, _shutdown| async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                if out.send(ClockMsg::Tick).is_err() {
                    break;
                }
            }
        });
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            ClockMsg::Tick => {
                self.time = Local::now().format(&self.format).to_string();
            }
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
