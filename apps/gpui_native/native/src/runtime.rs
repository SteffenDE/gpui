use crate::event::NativeEvent;
use std::sync::{Arc, Mutex};

#[cfg(feature = "real-gpui")]
use crate::{RasterData, WindowCommand};
#[cfg(feature = "real-gpui")]
use futures::channel::mpsc;
#[cfg(feature = "real-gpui")]
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        OnceLock,
    },
};

#[cfg(feature = "real-gpui")]
type FocusBinding = (Option<String>, Option<String>);
#[cfg(feature = "real-gpui")]
type FocusBindingKey = (u64, String);
#[cfg(feature = "real-gpui")]
type NonmodalPopovers = HashMap<(u64, String), NonmodalPopover>;

#[cfg(feature = "real-gpui")]
pub(crate) struct NonmodalPopover {
    pub(crate) open: std::sync::Weak<Mutex<bool>>,
    #[cfg(feature = "components")]
    pub(crate) order: u64,
    #[cfg(feature = "components")]
    pub(crate) parent: Option<String>,
    #[cfg(feature = "components")]
    pub(crate) hover: std::sync::Weak<Mutex<crate::component_registry::PopoverHoverState>>,
}

#[cfg(feature = "real-gpui")]
static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);
#[cfg(feature = "real-gpui")]
static GPUI_COMMANDS: OnceLock<mpsc::UnboundedSender<WindowCommand>> = OnceLock::new();

pub(crate) struct RuntimeState {
    pub(crate) events: Mutex<Vec<NativeEvent>>,
    #[cfg(feature = "components")]
    component_host: OnceLock<gpui_components::host::ComponentHost>,
    #[cfg(feature = "real-gpui")]
    pub(crate) resources: Mutex<HashMap<String, RasterData>>,
    #[cfg(feature = "real-gpui")]
    pub(crate) input_values: Mutex<HashMap<String, String>>,
    #[cfg(feature = "real-gpui")]
    pub(crate) focus_requests: Mutex<HashMap<(u64, String), u64>>,
    #[cfg(feature = "real-gpui")]
    pub(crate) clipboard_requests: Mutex<HashMap<(u64, String), u64>>,
    #[cfg(feature = "real-gpui")]
    pub(crate) focus_bindings: Mutex<HashMap<FocusBindingKey, FocusBinding>>,
    #[cfg(feature = "real-gpui")]
    pub(crate) focus_observers: Mutex<std::collections::HashSet<(u64, String)>>,
    #[cfg(feature = "real-gpui")]
    pub(crate) focus_handles: Mutex<HashMap<(u64, String), crate::gpui::FocusHandle>>,
    #[cfg(feature = "real-gpui")]
    pub(crate) element_bounds: Mutex<crate::element::event::BoundsCache>,
    #[cfg(feature = "real-gpui")]
    pub(crate) nonmodal_popovers: Mutex<NonmodalPopovers>,
    #[cfg(feature = "components")]
    pub(crate) next_popover_order: AtomicU64,
    #[cfg(feature = "real-gpui")]
    pub(crate) consumed_overlay_escape: Mutex<std::collections::HashSet<u64>>,
}

pub(crate) type SharedRuntime = Arc<RuntimeState>;

impl RuntimeState {
    #[cfg(feature = "components")]
    pub(crate) fn popover_owns_escape(&self, window_id: u64, id: &str) -> bool {
        if self
            .consumed_overlay_escape
            .lock()
            .is_ok_and(|windows| windows.contains(&window_id))
        {
            return false;
        }
        self.nonmodal_popovers.lock().is_ok_and(|popovers| {
            popovers
                .iter()
                .filter(|((window, _), state)| {
                    *window == window_id
                        && state
                            .open
                            .upgrade()
                            .is_some_and(|open| open.lock().is_ok_and(|value| *value))
                })
                .max_by_key(|(_, state)| state.order)
                .is_some_and(|((_, name), _)| name == id)
        })
    }

    #[cfg(feature = "components")]
    pub(crate) fn inside_popover_descendant(
        &self,
        window_id: u64,
        id: &str,
        position: crate::gpui::Point<crate::gpui::Pixels>,
    ) -> bool {
        self.nonmodal_popovers.lock().is_ok_and(|popovers| {
            popovers.iter().any(|((window, _), state)| {
                if *window != window_id
                    || !state
                        .open
                        .upgrade()
                        .is_some_and(|open| open.lock().is_ok_and(|value| *value))
                {
                    return false;
                }
                let inside = state.hover.upgrade().is_some_and(|hover| {
                    hover.lock().is_ok_and(|hover| {
                        hover
                            .content_bounds
                            .is_some_and(|bounds| bounds.contains(&position))
                    })
                });
                if !inside {
                    return false;
                }
                let mut parent = state.parent.as_deref();
                let mut visited = std::collections::HashSet::new();
                while let Some(name) = parent {
                    if name == id {
                        return true;
                    }
                    if !visited.insert(name) {
                        break;
                    }
                    parent = popovers
                        .get(&(window_id, name.to_string()))
                        .and_then(|state| state.parent.as_deref());
                }
                false
            })
        })
    }

    #[cfg(feature = "real-gpui")]
    pub(crate) fn overlay_owns_escape(&self, window_id: u64) -> bool {
        if self
            .consumed_overlay_escape
            .lock()
            .is_ok_and(|windows| windows.contains(&window_id))
        {
            return true;
        }
        self.nonmodal_popovers.lock().is_ok_and(|mut popovers| {
            popovers.retain(|_, state| state.open.strong_count() > 0);
            popovers.iter().any(|((window, _), state)| {
                *window == window_id
                    && state
                        .open
                        .upgrade()
                        .is_some_and(|open| open.lock().is_ok_and(|value| *value))
            })
        })
    }

    pub(crate) fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            #[cfg(feature = "components")]
            component_host: OnceLock::new(),
            #[cfg(feature = "real-gpui")]
            resources: Mutex::new(HashMap::new()),
            #[cfg(feature = "real-gpui")]
            input_values: Mutex::new(HashMap::new()),
            #[cfg(feature = "real-gpui")]
            focus_requests: Mutex::new(HashMap::new()),
            #[cfg(feature = "real-gpui")]
            clipboard_requests: Mutex::new(HashMap::new()),
            #[cfg(feature = "real-gpui")]
            focus_bindings: Mutex::new(HashMap::new()),
            #[cfg(feature = "real-gpui")]
            focus_observers: Mutex::new(std::collections::HashSet::new()),
            #[cfg(feature = "real-gpui")]
            focus_handles: Mutex::new(HashMap::new()),
            #[cfg(feature = "real-gpui")]
            element_bounds: Mutex::new(crate::element::event::BoundsCache::default()),
            #[cfg(feature = "real-gpui")]
            nonmodal_popovers: Mutex::new(HashMap::new()),
            #[cfg(feature = "components")]
            next_popover_order: AtomicU64::new(1),
            #[cfg(feature = "real-gpui")]
            consumed_overlay_escape: Mutex::new(std::collections::HashSet::new()),
        }
    }

    #[cfg(feature = "components")]
    pub(crate) fn component_host(self: &Arc<Self>) -> &gpui_components::host::ComponentHost {
        self.component_host.get_or_init(|| {
            gpui_components::host::ComponentHost::new(Arc::new(
                crate::component_host::NifComponentEventSink::new(self.clone()),
            ))
        })
    }
}

pub struct RuntimeResource {
    pub(crate) state: SharedRuntime,
    #[cfg(feature = "real-gpui")]
    pub(crate) id: u64,
    #[cfg(feature = "real-gpui")]
    pub(crate) command_tx: mpsc::UnboundedSender<WindowCommand>,
    #[cfg(feature = "real-gpui")]
    pub(crate) stopped: AtomicBool,
}

#[rustler::resource_impl]
impl rustler::Resource for RuntimeResource {}

impl RuntimeResource {
    #[cfg(feature = "real-gpui")]
    pub(crate) fn new() -> Result<Self, &'static str> {
        Ok(Self {
            state: Arc::new(RuntimeState::new()),
            id: NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed),
            command_tx: gpui_command_sender()?,
            stopped: AtomicBool::new(false),
        })
    }

    #[cfg(not(feature = "real-gpui"))]
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(RuntimeState::new()),
        }
    }
}

#[cfg(feature = "real-gpui")]
impl Drop for RuntimeResource {
    fn drop(&mut self) {
        if !self.stopped.swap(true, Ordering::AcqRel) {
            let _ = self
                .command_tx
                .unbounded_send(WindowCommand::ShutdownRuntime {
                    runtime_id: self.id,
                    reply: None,
                });
        }
    }
}

#[cfg(feature = "real-gpui")]
pub(crate) fn gpui_command_sender() -> Result<mpsc::UnboundedSender<WindowCommand>, &'static str> {
    if let Some(sender) = GPUI_COMMANDS.get() {
        return if sender.is_closed() {
            Err("gpui_runtime_stopped")
        } else {
            Ok(sender.clone())
        };
    }

    let (sender, receiver) = mpsc::unbounded();

    match GPUI_COMMANDS.set(sender.clone()) {
        Ok(()) => {
            crate::host::start(receiver, sender.clone())?;
            Ok(sender)
        }
        Err(_sender) => GPUI_COMMANDS
            .get()
            .filter(|sender| !sender.is_closed())
            .cloned()
            .ok_or("gpui_runtime_start_failed"),
    }
}

#[cfg(all(test, feature = "real-gpui"))]
mod tests {
    use super::*;

    #[test]
    fn dropping_runtime_requests_non_blocking_shutdown() {
        let (command_tx, mut commands) = mpsc::unbounded();
        let runtime = RuntimeResource {
            state: Arc::new(RuntimeState::new()),
            id: 42,
            command_tx,
            stopped: AtomicBool::new(false),
        };

        drop(runtime);

        match commands
            .try_recv()
            .expect("shutdown command should be queued")
        {
            WindowCommand::ShutdownRuntime {
                runtime_id: 42,
                reply: None,
            } => {}
            _other => panic!("unexpected runtime drop command"),
        }
    }
}
