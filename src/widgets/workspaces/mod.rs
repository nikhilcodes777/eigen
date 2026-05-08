use gtk::prelude::*;
pub mod provider;
use provider::get_provider;
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
    provider: Box<dyn provider::WorkspaceProvider + Send>,
    count: i32,
}

#[derive(Debug)]
pub enum WorkspacesMsg {
    ProviderEvent,
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
        let workspaces = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .detach();
        
        let mut provider = get_provider();
        let count_i32 = count as i32;
        let s = sender.clone();
        provider.start(Box::new(move || {
            s.input(WorkspacesMsg::ProviderEvent);
        }));
        
        let model = WorkspacesModel { workspaces, provider, count: count_i32 };
        let ws_factory_box = model.workspaces.widget();
        let widgets = view_output!();
        
        sender.input(WorkspacesMsg::ProviderEvent);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            WorkspacesMsg::ProviderEvent => {
                let ws_data = self.provider.get_workspaces(self.count);

                let current_len = self.workspaces.len();
                let target_len = ws_data.len();

                if target_len > current_len {
                    let mut guard = self.workspaces.guard();
                    for data in ws_data.iter().take(target_len).skip(current_len) {
                        guard.push_back(data.id as u8);
                    }
                } else if target_len < current_len {
                    let mut guard = self.workspaces.guard();
                    for _ in target_len..current_len {
                        guard.pop_back();
                    }
                }

                let mut updates = Vec::new();
                for (index, item) in self.workspaces.guard().iter().enumerate() {
                    if let Some(data) = ws_data.get(index)
                        && item.state != data.state {
                            updates.push((index, data.state));
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
