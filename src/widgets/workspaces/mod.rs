use gtk::prelude::*;
use hyprland::{
    data::{Workspace, Workspaces},
    event_listener::EventListener,
    shared::{HyprData, HyprDataActive},
};
use relm4::{factory::FactoryView, prelude::*};
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum WsState {
    Focused,
    Occupied,
    Unused,
}
#[derive(Debug)]
pub struct WorkspaceItem {
    state: WsState,
}
#[derive(Debug)]
pub enum WorkspaceItemMsg {
    UpdateState(WsState),
}

#[relm4::factory(pub)]
impl FactoryComponent for WorkspaceItem {
    type ParentWidget = gtk::Box;
    type Input = WorkspaceItemMsg;
    type Output = ();
    type Init = u8;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_valign: gtk::Align::Center,
            add_css_class: "ws-dot",
            #[watch]
            set_class_active: ("ws-focused", self.state == WsState::Focused),
            #[watch]
            set_class_active: ("ws-occupied", self.state == WsState::Occupied),
            #[watch]
            set_class_active: ("ws-unused", self.state == WsState::Unused),
         },
    }

    fn init_model(_init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            state: WsState::Unused,
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        _root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let widgets = view_output!();
        widgets
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {
            WorkspaceItemMsg::UpdateState(ws_state) => self.state = ws_state,
        }
    }
}

#[derive(Debug)]
pub struct WorkspacesModel {
    workspaces: FactoryVecDeque<WorkspaceItem>,
}

#[derive(Debug)]
pub enum WorkspacesMsg {
    HyprEvent,
}

#[relm4::component(pub)]
impl SimpleComponent for WorkspacesModel {
    type Input = WorkspacesMsg;
    type Output = ();
    type Init = u8;

    view! {
    #[root]
      gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 6,
                add_css_class: "workspaces-container",

                #[local_ref]
                ws_factory_box -> gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,
                }
            }
    }

    fn init(
        count: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut workspaces = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .detach();
        for id in 1..=count {
            workspaces.guard().push_back(id);
        }
        let model = WorkspacesModel { workspaces };
        let ws_factory_box = model.workspaces.widget();
        let widgets = view_output!();
        let s = sender.clone();
        std::thread::spawn(move || {
            let mut listener = EventListener::new();

            listener.add_workspace_changed_handler(move |_| {
                s.input(WorkspacesMsg::HyprEvent);
            });

            listener.start_listener().unwrap();
        });
        sender.input(WorkspacesMsg::HyprEvent);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            WorkspacesMsg::HyprEvent => {
                let active_id = Workspace::get_active().map(|w| w.id).unwrap_or(0);
                let all_workspaces = Workspaces::get().expect("Error getting all workspaces");
                let mut updates = Vec::new();

                for (index, item) in self.workspaces.guard().iter().enumerate() {
                    let ws_id = (index + 1) as i32;

                    let new_state = if ws_id == active_id {
                        WsState::Focused
                    } else if all_workspaces
                        .iter()
                        .any(|w| w.id == ws_id && w.windows > 0)
                    {
                        WsState::Occupied
                    } else {
                        WsState::Unused
                    };

                    if item.state != new_state {
                        updates.push((index, new_state));
                    }
                }
                for (index, new_state) in updates {
                    self.workspaces
                        .send(index, WorkspaceItemMsg::UpdateState(new_state));
                }
            }
        }
    }
}
