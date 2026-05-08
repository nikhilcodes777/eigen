use std::env;

use crate::widgets::workspaces::WsState;

#[derive(Debug)]
pub struct WorkspaceData {
    pub id: i32,
    pub state: WsState,
}

pub trait WorkspaceProvider {
    fn start(&mut self, on_update: Box<dyn Fn() + Send + 'static>);
    fn get_workspaces(&self, count: i32) -> Vec<WorkspaceData>;
}

impl std::fmt::Debug for dyn WorkspaceProvider + Send {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WorkspaceProvider")
    }
}

pub fn get_provider() -> Box<dyn WorkspaceProvider + Send> {
    if env::var("NIRI_SOCKET").is_ok() {
        Box::new(niri_provider::NiriProvider::new())
    } else {
        Box::new(hyprland_provider::HyprlandProvider::new())
    }
}

pub mod hyprland_provider {
    use hyprland::{
        data::{Workspace, Workspaces},
        event_listener::EventListener,
        shared::{HyprData, HyprDataActive},
    };
    use super::{WorkspaceData, WorkspaceProvider, WsState};

    pub struct HyprlandProvider;

    impl HyprlandProvider {
        pub fn new() -> Self {
            Self
        }
    }

    impl WorkspaceProvider for HyprlandProvider {
        fn start(&mut self, on_update: Box<dyn Fn() + Send + 'static>) {
            std::thread::spawn(move || {
                let mut listener = EventListener::new();

                let update_fn = std::sync::Arc::new(on_update);
                
                let u1 = update_fn.clone();
                listener.add_workspace_changed_handler(move |_| u1());
                
                let u2 = update_fn.clone();
                listener.add_window_closed_handler(move |_| u2());
                
                let u3 = update_fn.clone();
                listener.add_window_opened_handler(move |_| u3());
                
                let u4 = update_fn.clone();
                listener.add_active_window_changed_handler(move |_| u4());

                listener.start_listener().unwrap();
            });
        }

        fn get_workspaces(&self, count: i32) -> Vec<WorkspaceData> {
            let active_id = Workspace::get_active().map(|w| w.id).unwrap_or(0);
            let all_workspaces = Workspaces::get().expect("Error getting all workspaces");

            (1..=count)
                .map(|ws_id| {
                    let state = if ws_id == active_id {
                        WsState::Focused
                    } else if all_workspaces.iter().any(|w| w.id == ws_id && w.windows > 0) {
                        WsState::Occupied
                    } else {
                        WsState::Unused
                    };
                    WorkspaceData { id: ws_id, state }
                })
                .collect()
        }
    }
}

pub mod niri_provider {
    use niri_ipc::socket::Socket;
    use niri_ipc::state::{EventStreamState, EventStreamStatePart};
    use niri_ipc::{Request, Response};
    use std::sync::{Arc, Mutex};
    use super::{WorkspaceData, WorkspaceProvider, WsState};

    pub struct NiriProvider {
        state: Arc<Mutex<EventStreamState>>,
    }

    impl NiriProvider {
        pub fn new() -> Self {
            Self {
                state: Arc::new(Mutex::new(EventStreamState::default())),
            }
        }
    }

    impl WorkspaceProvider for NiriProvider {
        fn start(&mut self, on_update: Box<dyn Fn() + Send + 'static>) {
            let state = self.state.clone();
            std::thread::spawn(move || {
                let mut socket = match Socket::connect() {
                    Ok(s) => s,
                    Err(_) => return,
                };

                if let Ok(Ok(Response::Handled)) = socket.send(Request::EventStream) {
                    let mut read_event = socket.read_events();
                    while let Ok(event) = read_event() {
                        {
                            let mut st = state.lock().unwrap();
                            st.apply(event);
                        }
                        on_update();
                    }
                }
            });
        }

        fn get_workspaces(&self, _count: i32) -> Vec<WorkspaceData> {
            let st = self.state.lock().unwrap();
            
            let mut workspaces: Vec<_> = st.workspaces.workspaces.values().collect();
            workspaces.sort_by_key(|w| w.idx);
            
            workspaces
                .into_iter()
                .map(|ws| {
                    let state = if ws.is_active || ws.is_focused {
                        WsState::Focused
                    } else {
                        let has_windows = st.windows.windows.values().any(|win| win.workspace_id == Some(ws.id));
                        if has_windows {
                            WsState::Occupied
                        } else {
                            WsState::Unused
                        }
                    };
                    
                    WorkspaceData { id: (ws.idx + 1) as i32, state }
                })
                .collect()
        }
    }
}
