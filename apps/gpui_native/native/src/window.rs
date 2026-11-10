use crate::*;

#[cfg(feature = "real-gpui")]
pub(crate) type WindowCommandReply = std::sync::mpsc::SyncSender<Result<(), String>>;

#[cfg(feature = "real-gpui")]
pub(crate) type WindowGenerationReply = std::sync::mpsc::SyncSender<Result<u64, String>>;

#[cfg(feature = "real-gpui")]
#[derive(Clone)]
pub(crate) struct CommandBinding {
    id: String,
    shortcut: gpui::KeybindingKeystroke,
}

#[cfg(feature = "real-gpui")]
impl CommandBinding {
    pub(crate) fn new(id: String, shortcut: String) -> Result<Self, &'static str> {
        if !gpui_core::window_codec::valid_command(&id, &shortcut) {
            return Err("invalid_command");
        }

        let shortcut = shortcut
            .split('-')
            .map(|part| if part == "primary" { "secondary" } else { part })
            .collect::<Vec<_>>()
            .join("-");
        let shortcut = gpui::Keystroke::parse(&shortcut).map_err(|_error| "invalid_command")?;

        Ok(Self {
            id,
            shortcut: gpui::KeybindingKeystroke::from_keystroke(shortcut),
        })
    }

    fn matches(&self, keystroke: &gpui::Keystroke) -> bool {
        keystroke.should_match(&self.shortcut)
    }
}

#[cfg(feature = "real-gpui")]
fn native_editing_shortcut(keystroke: &gpui::Keystroke) -> bool {
    let modifiers = &keystroke.modifiers;
    let text_key = !modifiers.platform && !modifiers.control && !modifiers.alt;
    let editing_command = (modifiers.platform || modifiers.control)
        && !modifiers.alt
        && matches!(
            keystroke.key.as_str(),
            "a" | "c" | "v" | "x" | "y" | "z" | "left" | "right"
        );
    text_key || editing_command
}

#[cfg(feature = "real-gpui")]
fn root_capture_matches(keystroke: &gpui::Keystroke, keys: &[String], input_focused: bool) -> bool {
    let modifiers = keystroke.modifiers;
    if input_focused
        || modifiers.control
        || modifiers.alt
        || modifiers.platform
        || modifiers.function
    {
        return false;
    }
    keys.iter().any(|binding| {
        if modifiers.shift {
            binding.strip_prefix("shift-") == Some(keystroke.key.as_str())
        } else {
            binding == &keystroke.key
        }
    })
}

#[cfg(feature = "real-gpui")]
fn root_key_capture(tree: &ElementNode) -> Option<(String, Vec<String>)> {
    match tree {
        ElementNode::Div(node) => node
            .keycapture
            .as_ref()
            .map(|event| (event.clone(), node.capture_keys.clone())),
        ElementNode::Viewport(node) => node.children.first().and_then(root_key_capture),
        _ => None,
    }
}

#[cfg(feature = "real-gpui")]
#[derive(Clone)]
struct RootInputEvents {
    keyrelease: Option<String>,
    pointer_position: Option<String>,
    pointer_up: Option<String>,
    runtime: SharedRuntime,
    window_id: u64,
}

#[cfg(feature = "real-gpui")]
fn root_input_events(
    tree: &ElementNode,
    runtime: SharedRuntime,
    window_id: u64,
) -> Option<RootInputEvents> {
    match tree {
        ElementNode::Div(node) => Some(RootInputEvents {
            keyrelease: node.window_keyrelease.clone(),
            pointer_position: node.window_pointer_position.clone(),
            pointer_up: node.window_pointer_up.clone(),
            runtime,
            window_id,
        }),
        ElementNode::Viewport(node) => node
            .children
            .first()
            .and_then(|tree| root_input_events(tree, runtime, window_id)),
        _ => None,
    }
}

#[cfg(all(test, feature = "real-gpui"))]
mod command_tests {
    use super::{native_editing_shortcut, root_capture_matches, CommandBinding};

    #[test]
    fn explicit_shift_capture_preserves_editing_and_unlisted_modifiers() {
        let keys = vec!["r".into(), "shift-c".into(), "shift-enter".into()];
        for key in ["r", "shift-c", "shift-enter"] {
            let keystroke = zed_gpui::Keystroke::parse(key).unwrap();
            assert!(root_capture_matches(&keystroke, &keys, false));
            assert!(!root_capture_matches(&keystroke, &keys, true));
        }
        for key in ["c", "shift-r", "secondary-r", "ctrl-r", "alt-r"] {
            let keystroke = zed_gpui::Keystroke::parse(key).unwrap();
            assert!(!root_capture_matches(&keystroke, &keys, false));
        }
    }

    #[test]
    fn primary_shortcuts_use_gpui_platform_matching() {
        let command = CommandBinding::new("refresh".to_string(), "primary-r".to_string())
            .expect("valid command");
        let matching = zed_gpui::Keystroke::parse("secondary-r").expect("valid keystroke");
        let unmodified = zed_gpui::Keystroke::parse("r").expect("valid keystroke");

        assert!(command.matches(&matching));
        assert!(!command.matches(&unmodified));
    }

    #[test]
    fn native_editing_shortcuts_are_reserved_while_an_input_is_focused() {
        let select_all = zed_gpui::Keystroke::parse("ctrl-a").expect("valid keystroke");
        let refresh = zed_gpui::Keystroke::parse("ctrl-r").expect("valid keystroke");

        assert!(native_editing_shortcut(&select_all));
        assert!(!native_editing_shortcut(&refresh));
        for key in [
            "secondary-left",
            "secondary-right",
            "ctrl-left",
            "ctrl-right",
        ] {
            let keystroke = zed_gpui::Keystroke::parse(key).expect("valid cursor shortcut");
            assert!(native_editing_shortcut(&keystroke));
        }
        for key in ["s", "shift-r", "1", "enter", "down"] {
            let keystroke = zed_gpui::Keystroke::parse(key).expect("valid keystroke");
            assert!(native_editing_shortcut(&keystroke));
            let binding = CommandBinding::new("command".into(), key.into()).expect("valid binding");
            assert!(binding.matches(&keystroke));
        }
    }
}

#[cfg(feature = "real-gpui")]
pub(crate) struct WindowState {
    pub(crate) tree: Mutex<ElementNode>,
    commands: Vec<CommandBinding>,
    requested_generation: std::sync::atomic::AtomicU64,
    rendered_generation: std::sync::atomic::AtomicU64,
    scheduled_frame_generation: std::sync::atomic::AtomicU64,
    completed_frame_generation: std::sync::atomic::AtomicU64,
    frame_waiters: Mutex<Vec<(u64, WindowCommandReply)>>,
    next_frame_waiters: Mutex<Vec<(u64, WindowCommandReply)>>,
}

#[cfg(feature = "real-gpui")]
impl WindowState {
    pub(crate) fn new(tree: ElementNode, commands: Vec<CommandBinding>) -> Self {
        Self {
            tree: Mutex::new(tree),
            commands,
            requested_generation: std::sync::atomic::AtomicU64::new(1),
            rendered_generation: std::sync::atomic::AtomicU64::new(0),
            scheduled_frame_generation: std::sync::atomic::AtomicU64::new(0),
            completed_frame_generation: std::sync::atomic::AtomicU64::new(0),
            frame_waiters: Mutex::new(Vec::new()),
            next_frame_waiters: Mutex::new(Vec::new()),
        }
    }
}

#[cfg(feature = "real-gpui")]
pub(crate) type SharedWindow = Arc<WindowState>;

#[cfg(all(feature = "real-gpui", feature = "components"))]
type DialogKeyHandler = Arc<dyn Fn(&gpui::KeyDownEvent, &mut gpui::Window, &mut gpui::App)>;

#[cfg(feature = "real-gpui")]
pub(crate) struct ElixirRoot {
    pub(crate) window_state: SharedWindow,
    pub(crate) runtime: SharedRuntime,
    pub(crate) pointer_observers: crate::element::event::PointerObservers,
    pointer_focus: Option<gpui::FocusHandle>,
    pub(crate) scrolls: crate::element::scroll::Scrolls,
    #[cfg(feature = "components")]
    pub(crate) horizontal_groups: crate::element::component::horizontal::Groups,
    pub(crate) window_id: u64,
    input_entities: HashMap<String, gpui::Entity<NativeTextInput>>,
    terminal_entities: HashMap<String, gpui::Entity<crate::element::component::terminal::Terminal>>,
    #[cfg(feature = "components")]
    pub(crate) components: crate::element::component_registry::ComponentRegistry,
    command_subscription: Option<gpui::Subscription>,
    activation_subscription: Option<gpui::Subscription>,
    appearance_subscription: Option<gpui::Subscription>,
    geometry_subscription: Option<gpui::Subscription>,
    geometry_window: bool,
    last_geometry: Option<gpui::Bounds<gpui::Pixels>>,
    pub(crate) appearance_window: bool,
    focus_window: bool,
    blur_window: bool,
    observe_commands: bool,
    #[cfg(feature = "components")]
    render_dialog_layer: bool,
    #[cfg(feature = "components")]
    dialog_focus: Option<gpui::FocusHandle>,
    #[cfg(feature = "components")]
    dialog_key_handler: Option<DialogKeyHandler>,
}

#[cfg(feature = "real-gpui")]
impl ElixirRoot {
    #[cfg(feature = "components")]
    pub(crate) fn render_tooltip_children(
        &mut self,
        nodes: Vec<crate::ElementNode>,
        id_namespace: String,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        let mut active_input_ids = HashSet::new();
        let mut context = ElementRenderContext {
            pointer_observers: self.pointer_observers.clone(),
            scrolls: self.scrolls.clone(),
            horizontal_groups: self.horizontal_groups.clone(),
            tooltip_focus_owner: None,
            runtime: self.runtime.clone(),
            window_id: self.window_id,
            next_element_id: 0,
            id_namespace,
            active_input_ids: &mut active_input_ids,
            input_entities: &mut self.input_entities,
            terminal_entities: &mut self.terminal_entities,
            components: &mut self.components,
            window,
            cx,
        };
        nodes
            .into_iter()
            .map(|node| node.render(&mut context))
            .collect()
    }

    pub(crate) fn new(
        window_state: SharedWindow,
        runtime: SharedRuntime,
        window_id: u64,
        focus_window: bool,
        blur_window: bool,
    ) -> Self {
        Self {
            window_state,
            runtime,
            window_id,
            pointer_observers: Default::default(),
            pointer_focus: None,
            scrolls: Default::default(),
            #[cfg(feature = "components")]
            horizontal_groups: Default::default(),
            input_entities: HashMap::new(),
            terminal_entities: HashMap::new(),
            #[cfg(feature = "components")]
            components: crate::element::component_registry::ComponentRegistry::default(),
            command_subscription: None,
            activation_subscription: None,
            appearance_subscription: None,
            geometry_subscription: None,
            geometry_window: false,
            last_geometry: None,
            appearance_window: false,
            focus_window,
            blur_window,
            observe_commands: true,
            #[cfg(feature = "components")]
            render_dialog_layer: true,
            #[cfg(feature = "components")]
            dialog_focus: None,
            #[cfg(feature = "components")]
            dialog_key_handler: None,
        }
    }

    #[cfg(feature = "components")]
    pub(crate) fn new_dialog(
        window_state: SharedWindow,
        runtime: SharedRuntime,
        window_id: u64,
        focus: gpui::FocusHandle,
        key_handler: DialogKeyHandler,
    ) -> Self {
        let mut root = Self::new(window_state, runtime, window_id, false, false);
        root.render_dialog_layer = false;
        root.observe_commands = false;
        root.dialog_focus = Some(focus);
        root.dialog_key_handler = Some(key_handler);
        root
    }

    fn report_appearance(&self, window: &mut gpui::Window, cx: &mut gpui::App) {
        #[cfg(feature = "components")]
        if cx.has_global::<gpui_component::Theme>() {
            gpui_component::Theme::sync_system_appearance(Some(&mut *window), cx);
        }
        #[cfg(not(feature = "components"))]
        let _ = cx;
        if self.appearance_window {
            let dark = matches!(
                window.appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            );
            let _ = push_event(
                &self.runtime,
                NativeEvent::WindowAppearance {
                    window_id: self.window_id,
                    dark,
                },
            );
        }
    }

    fn report_geometry(&mut self, window: &gpui::Window, initial: bool) {
        if !self.geometry_window || window.is_fullscreen() {
            return;
        }
        let bounds = window.bounds();
        if bounds.size.width <= gpui::px(0.0)
            || bounds.size.height <= gpui::px(0.0)
            || self.last_geometry == Some(bounds)
        {
            return;
        }
        self.last_geometry = Some(bounds);
        let _ = push_event(
            &self.runtime,
            NativeEvent::WindowGeometry {
                window_id: self.window_id,
                bounds: vec![
                    f32::from(bounds.origin.x) as f64,
                    f32::from(bounds.origin.y) as f64,
                    f32::from(bounds.size.width) as f64,
                    f32::from(bounds.size.height) as f64,
                ],
                initial,
            },
        );
    }

    fn observe_geometry(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if self.geometry_subscription.is_some() || !self.geometry_window {
            return;
        }
        self.geometry_subscription = Some(cx.observe_window_bounds(window, |root, window, _cx| {
            root.report_geometry(window, false);
        }));
        self.report_geometry(window, true);
    }

    fn observe_appearance(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if self.appearance_subscription.is_some() || !self.observe_commands {
            return;
        }
        self.appearance_subscription =
            Some(cx.observe_window_appearance(window, |root, window, cx| {
                root.report_appearance(window, cx);
                cx.notify();
            }));
        self.report_appearance(window, cx);
    }

    fn observe_window_activation(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.activation_subscription.is_some() || (!self.focus_window && !self.blur_window) {
            return;
        }

        let runtime = self.runtime.clone();
        let window_id = self.window_id;
        let focus_window = self.focus_window;
        let blur_window = self.blur_window;
        self.activation_subscription = Some(cx.observe_window_activation(
            window,
            move |_root, window, _cx| {
                let focused = window.is_window_active();
                if (focused && focus_window) || (!focused && blur_window) {
                    let _ = push_event(&runtime, NativeEvent::WindowFocus { focused, window_id });
                }
            },
        ));
        let focused = window.is_window_active();
        if (focused && focus_window) || (!focused && blur_window) {
            let _ = push_event(
                &self.runtime,
                NativeEvent::WindowFocus { focused, window_id },
            );
        }
    }

    fn editable_input_focused(&self, window: &gpui::Window, cx: &gpui::App) -> bool {
        use gpui::Focusable;

        let primitive_input_focused = self
            .input_entities
            .values()
            .any(|input| input.read(cx).focus_handle(cx).is_focused(window));

        #[cfg(feature = "components")]
        let component_input_focused = self.components.editable_input_focused(window, cx);
        #[cfg(not(feature = "components"))]
        let component_input_focused = false;

        let terminal_focused = self
            .terminal_entities
            .values()
            .any(|terminal| terminal.read(cx).input_focused(window));
        primitive_input_focused || component_input_focused || terminal_focused
    }

    fn observe_commands(&mut self, window: &gpui::Window, cx: &mut gpui::Context<Self>) {
        if !self.observe_commands || self.command_subscription.is_some() {
            return;
        }

        let expected_window = window.window_handle();
        let commands = self.window_state.commands.clone();
        let runtime = self.runtime.clone();
        let window_id = self.window_id;
        let view = cx.weak_entity();
        self.command_subscription = Some(cx.intercept_keystrokes(move |event, window, cx| {
            let Some(view) = view.upgrade() else {
                return;
            };
            view.update(cx, |root, cx| {
                if window.window_handle() != expected_window {
                    return;
                }
                if event.keystroke.key == "escape" && runtime.overlay_owns_escape(window_id) {
                    return;
                }
                let input_focused = root.editable_input_focused(window, cx)
                    || event
                        .context_stack
                        .iter()
                        .any(|context| context.contains("GPUIInput") || context.contains("Input"));
                let capture = root
                    .window_state
                    .tree
                    .lock()
                    .ok()
                    .and_then(|tree| root_key_capture(&tree));
                if let Some((callback, keys)) = capture {
                    let modifiers = event.keystroke.modifiers;
                    let captured = root_capture_matches(&event.keystroke, &keys, input_focused);
                    let _ = push_event(
                        &runtime,
                        NativeEvent::CapturedKeystroke {
                            window_id,
                            event: callback,
                            captured,
                            value: crate::event::KeyStroke {
                                key: event.keystroke.key.clone(),
                                control: modifiers.control,
                                alt: modifiers.alt,
                                shift: modifiers.shift,
                                platform: modifiers.platform,
                                repeat: false,
                            },
                        },
                    );
                    if captured {
                        cx.stop_propagation();
                        return;
                    }
                }
                if native_editing_shortcut(&event.keystroke) && input_focused {
                    return;
                }

                if let Some(command) = commands
                    .iter()
                    .find(|command| command.matches(&event.keystroke))
                {
                    cx.stop_propagation();
                    let _ = push_event(
                        &runtime,
                        NativeEvent::Command {
                            window_id,
                            event: command.id.clone(),
                        },
                    );
                }
            });
        }));
    }
}

#[cfg(all(feature = "real-gpui", feature = "components"))]
pub(crate) fn bind_dialog_content_keys(cx: &mut gpui::App) {
    // The component library binds Enter to confirm the whole dialog. Elixir
    // dialogs delegate activation to their focused controls instead.
    cx.bind_keys([gpui::KeyBinding::new(
        "enter",
        gpui::NoAction {},
        Some("GPUIElixirDialogContent"),
    )]);
}

#[cfg(feature = "real-gpui")]
fn root_rem_size(tree: &ElementNode) -> f64 {
    if let ElementNode::Viewport(node) = tree {
        if let Some(size) = node.rem_size {
            return size;
        }
        if node.children.len() == 1 {
            return root_rem_size(&node.children[0]);
        }
    }
    16.0
}

#[cfg(feature = "real-gpui")]
impl gpui::Render for ElixirRoot {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let pointer_focus = self
            .pointer_focus
            .get_or_insert_with(|| cx.focus_handle().tab_stop(false))
            .clone();
        if _window.focused(cx).is_none() {
            pointer_focus.focus(_window, cx);
        }
        self.observe_commands(_window, cx);
        self.observe_window_activation(_window, cx);
        self.observe_geometry(_window, cx);
        self.observe_appearance(_window, cx);

        let tree = self
            .window_state
            .tree
            .lock()
            .map(|tree| tree.clone())
            .unwrap_or_else(|_| ElementNode::empty_root());
        if self.observe_commands {
            _window.set_rem_size(gpui::px(root_rem_size(&tree) as f32));
        }
        #[cfg(feature = "components")]
        let owns_bounds_frame = self.render_dialog_layer;
        #[cfg(not(feature = "components"))]
        let owns_bounds_frame = true;
        if owns_bounds_frame {
            if let Ok(mut bounds) = self.runtime.element_bounds.lock() {
                bounds.begin_frame(self.window_id);
            }
        }
        let input_events = if self.observe_commands {
            root_input_events(&tree, self.runtime.clone(), self.window_id)
        } else {
            None
        };
        let id_namespace = format!("root-{:?}", cx.entity_id());
        let mut active_input_ids = HashSet::new();
        #[cfg(feature = "components")]
        self.components.begin_render();
        self.pointer_observers.borrow_mut().clear();
        crate::element::scroll::begin_frame(&self.scrolls);
        #[cfg(feature = "components")]
        crate::element::component::horizontal::begin_frame(&self.horizontal_groups);
        let element = tree.render(&mut ElementRenderContext {
            pointer_observers: self.pointer_observers.clone(),
            scrolls: self.scrolls.clone(),
            #[cfg(feature = "components")]
            horizontal_groups: self.horizontal_groups.clone(),
            #[cfg(feature = "components")]
            tooltip_focus_owner: None,
            runtime: self.runtime.clone(),
            window_id: self.window_id,
            next_element_id: 0,
            id_namespace: id_namespace.clone(),
            active_input_ids: &mut active_input_ids,
            input_entities: &mut self.input_entities,
            terminal_entities: &mut self.terminal_entities,
            #[cfg(feature = "components")]
            components: &mut self.components,
            window: _window,
            cx,
        });
        self.input_entities
            .retain(|input_id, _entity| active_input_ids.contains(input_id));
        self.terminal_entities
            .retain(|id, _entity| active_input_ids.contains(id));
        #[cfg(feature = "components")]
        self.components.finish_render(_window, cx);

        let input_prefix = format!("gpui-elixir-input-{}-{}-", self.window_id, id_namespace);
        if let Ok(mut input_values) = self.runtime.input_values.lock() {
            input_values.retain(|input_id, _value| {
                !input_id.starts_with(&input_prefix) || active_input_ids.contains(input_id)
            });
        }

        #[cfg(feature = "components")]
        if self.render_dialog_layer {
            use gpui::{IntoElement, ParentElement};

            let element = gpui::div()
                .child(element)
                .children(gpui_component::Root::render_dialog_layer(_window, cx))
                .into_any_element();
            return acknowledge_frame(
                element,
                self.window_state.clone(),
                self.pointer_observers.clone(),
                pointer_focus,
                input_events,
            );
        }

        #[cfg(feature = "components")]
        if let (Some(focus), Some(key_handler)) =
            (self.dialog_focus.clone(), self.dialog_key_handler.clone())
        {
            use gpui::{InteractiveElement, IntoElement, ParentElement};

            let element = gpui::div()
                .id("gpui-elixir-dialog-content")
                .key_context("GPUIElixirDialogContent")
                .track_focus(&focus.tab_stop(true))
                .on_key_down(move |event, window, cx| key_handler(event, window, cx))
                .child(element)
                .into_any_element();
            return acknowledge_frame(
                element,
                self.window_state.clone(),
                self.pointer_observers.clone(),
                pointer_focus,
                input_events,
            );
        }

        acknowledge_frame(
            element,
            self.window_state.clone(),
            self.pointer_observers.clone(),
            pointer_focus,
            input_events,
        )
    }
}

#[cfg(feature = "real-gpui")]
fn acknowledge_frame(
    element: gpui::AnyElement,
    window_state: SharedWindow,
    pointer_observers: crate::element::event::PointerObservers,
    pointer_focus: gpui::FocusHandle,
    input_events: Option<RootInputEvents>,
) -> gpui::AnyElement {
    use gpui::{InteractiveElement, IntoElement, ParentElement, Styled};
    use std::sync::atomic::Ordering;

    let barrier = gpui::canvas(
        move |_bounds, window, _cx| {
            let generation = window_state.requested_generation.load(Ordering::Acquire);
            let frame_generation = window_state
                .scheduled_frame_generation
                .fetch_add(1, Ordering::AcqRel)
                + 1;
            let window_state = window_state.clone();
            window.on_next_frame(move |_window, _cx| {
                complete_frame(&window_state, generation, frame_generation)
            });
        },
        {
            let input_events = input_events.clone();
            move |_bounds, _prepaint, window, _cx| {
                if let Some(input) = &input_events {
                    if let Some(callback) = &input.pointer_position {
                        let (event, runtime, window_id) =
                            (callback.clone(), input.runtime.clone(), input.window_id);
                        window.on_mouse_event(move |mouse: &gpui::MouseMoveEvent, phase, _, _| {
                            if phase == gpui::DispatchPhase::Capture {
                                crate::element::event::emit_pointer_position(
                                    &runtime,
                                    window_id,
                                    &event,
                                    mouse.position,
                                );
                            }
                        });
                    }
                    if let Some(callback) = &input.pointer_up {
                        let (event, runtime, window_id) =
                            (callback.clone(), input.runtime.clone(), input.window_id);
                        window.on_mouse_event(move |mouse: &gpui::MouseUpEvent, phase, _, _| {
                            if phase == gpui::DispatchPhase::Capture
                                && mouse.button == gpui::MouseButton::Left
                            {
                                crate::element::event::emit_pointer_position(
                                    &runtime,
                                    window_id,
                                    &event,
                                    mouse.position,
                                );
                            }
                        });
                    }
                }
            }
        },
    )
    .absolute()
    .size_full();

    let mut root = gpui::div();
    if let Some(input) = input_events {
        if let Some(event) = input.keyrelease {
            let (runtime, window_id) = (input.runtime, input.window_id);
            root = root.capture_key_up(move |key, _, _| {
                crate::element::event::emit_structured_key(
                    &runtime,
                    window_id,
                    &event,
                    &key.keystroke,
                    false,
                );
            });
        }
    }
    root.relative()
        .size_full()
        .track_focus(&pointer_focus)
        .on_modifiers_changed(move |event, _, cx| {
            crate::element::event::dispatch_pointer_modifiers(
                &pointer_observers,
                event.modifiers,
                cx,
            )
        })
        .child(barrier)
        .child(element)
        .into_any_element()
}

#[cfg(feature = "real-gpui")]
fn complete_frame(window_state: &SharedWindow, generation: u64, frame_generation: u64) {
    use std::sync::atomic::Ordering;

    window_state
        .rendered_generation
        .fetch_max(generation, Ordering::AcqRel);
    window_state
        .completed_frame_generation
        .fetch_max(frame_generation, Ordering::AcqRel);

    complete_waiters(&window_state.frame_waiters, generation);
    complete_waiters(&window_state.next_frame_waiters, frame_generation);
}

#[cfg(feature = "real-gpui")]
fn complete_waiters(waiters: &Mutex<Vec<(u64, WindowCommandReply)>>, generation: u64) {
    if let Ok(mut waiters) = waiters.lock() {
        let mut pending = Vec::new();
        for (target, reply) in std::mem::take(&mut *waiters) {
            if target <= generation {
                send_reply(reply, Ok(()));
            } else {
                pending.push((target, reply));
            }
        }
        *waiters = pending;
    }
}

#[cfg(feature = "real-gpui")]
fn await_generation(
    completed: &std::sync::atomic::AtomicU64,
    waiters: &Mutex<Vec<(u64, WindowCommandReply)>>,
    target: u64,
    reply: WindowCommandReply,
) {
    use std::sync::atomic::Ordering;

    if completed.load(Ordering::Acquire) >= target {
        send_reply(reply, Ok(()));
        return;
    }

    match waiters.lock() {
        Ok(mut waiters) => {
            if completed.load(Ordering::Acquire) >= target {
                send_reply(reply, Ok(()));
            } else {
                waiters.push((target, reply));
            }
        }
        Err(_error) => send_reply(reply, Err("runtime_lock_failed".to_string())),
    }
}

#[cfg(feature = "real-gpui")]
#[derive(Clone, Copy)]
pub(crate) enum NativeThemeMode {
    System,
    Light,
    Dark,
}

#[cfg(feature = "real-gpui")]
pub(crate) enum WindowCommand {
    Open {
        runtime_id: u64,
        title: String,
        window_id: u64,
        width: f32,
        height: f32,
        min_size: Option<(f32, f32)>,
        restore_bounds: Option<Vec<f64>>,
        cascade_offset: u32,
        center_in_work_area: bool,
        geometry: bool,
        resizable: bool,
        content_chrome: bool,
        close_request: bool,
        focus: bool,
        blur: bool,
        appearance: bool,
        window_state: SharedWindow,
        runtime: SharedRuntime,
        reply: WindowCommandReply,
    },
    Update {
        runtime_id: u64,
        window_id: u64,
        tree: Box<ElementNode>,
        reply: WindowCommandReply,
    },
    Close {
        runtime_id: u64,
        window_id: u64,
        reply: WindowCommandReply,
    },
    AwaitFrame {
        runtime_id: u64,
        window_id: u64,
        reply: WindowCommandReply,
    },
    FrameToken {
        runtime_id: u64,
        window_id: u64,
        reply: WindowGenerationReply,
    },
    AwaitFrameAfter {
        runtime_id: u64,
        window_id: u64,
        generation: u64,
        reply: WindowCommandReply,
    },
    ShutdownRuntime {
        runtime_id: u64,
        reply: Option<WindowCommandReply>,
    },
    ShowSystemNotification {
        tag: String,
        title: String,
        subtitle: String,
        body: String,
        sound: bool,
        reply: WindowCommandReply,
    },
    DismissSystemNotification {
        tag: String,
        reply: WindowCommandReply,
    },
    SubscribeSystemNotifications {
        pid: rustler::LocalPid,
        reply: WindowCommandReply,
    },
    SystemNotificationPermission {
        request: bool,
        reply: std::sync::mpsc::SyncSender<Result<String, String>>,
    },
    ActivateWindow {
        runtime_id: u64,
        window_id: u64,
        reply: WindowCommandReply,
    },
    SetAppIdentity {
        identifier: String,
        name: String,
        reply: WindowCommandReply,
    },
    LoadFont {
        bytes: Vec<u8>,
        reply: WindowCommandReply,
    },
    SetTheme {
        mode: NativeThemeMode,
        reply: WindowCommandReply,
    },
    PlatformClosed {
        platform_id: gpui::WindowId,
    },
}

#[cfg(all(feature = "real-gpui", feature = "components"))]
type NativeWindowRoot = gpui_component::Root;

#[cfg(all(feature = "real-gpui", not(feature = "components")))]
type NativeWindowRoot = ElixirRoot;

#[cfg(feature = "real-gpui")]
struct ManagedWindow {
    handle: gpui::WindowHandle<NativeWindowRoot>,
    view: gpui::Entity<ElixirRoot>,
    state: SharedWindow,
    runtime: SharedRuntime,
}

#[cfg(feature = "real-gpui")]
type WindowKey = (u64, u64);

#[cfg(feature = "real-gpui")]
pub(crate) fn run_gpui(
    mut commands: mpsc::UnboundedReceiver<WindowCommand>,
    command_tx: mpsc::UnboundedSender<WindowCommand>,
    ready: std::sync::mpsc::SyncSender<Result<(), &'static str>>,
) {
    use gpui::App;

    let application = gpui_platform::application().with_quit_mode(gpui::QuitMode::Explicit);

    #[cfg(feature = "components")]
    let application = application.with_assets(gpui_component_assets::Assets);

    application.run(move |cx: &mut App| {
        #[cfg(feature = "components")]
        {
            gpui_component::init(cx);
            bind_dialog_content_keys(cx);
        }

        bind_input_keys(cx);
        crate::element::component::terminal::bind_terminal_keys(cx);

        let window_closed_subscription = cx.on_window_closed(move |_cx, platform_id| {
            let _ = command_tx.unbounded_send(WindowCommand::PlatformClosed { platform_id });
        });

        cx.spawn(async move |cx| {
            let _window_closed_subscription = window_closed_subscription;
            let mut windows = HashMap::<WindowKey, ManagedWindow>::new();
            let mut identity: Option<(String, String)> = None;

            while let Some(command) = commands.next().await {
                cx.update(|cx| handle_window_command(command, &mut windows, &mut identity, cx));
            }
        })
        .detach();

        cx.activate(true);
        let _ = ready.send(Ok(()));
    });
}

#[cfg(feature = "real-gpui")]
fn handle_window_command(
    command: WindowCommand,
    windows: &mut HashMap<WindowKey, ManagedWindow>,
    identity: &mut Option<(String, String)>,
    cx: &mut gpui::App,
) {
    match command {
        WindowCommand::Open {
            runtime_id,
            title,
            window_id,
            width,
            height,
            min_size,
            restore_bounds,
            cascade_offset,
            center_in_work_area,
            geometry,
            resizable,
            content_chrome,
            close_request,
            focus,
            blur,
            appearance,
            window_state,
            runtime,
            reply,
        } => {
            let key = (runtime_id, window_id);
            if let Some(window) = windows.remove(&key) {
                let _ = close_managed_window(window, cx);
            }

            let result = open_gpui_window(
                WindowOpenConfig {
                    title,
                    window_id,
                    width,
                    height,
                    min_size,
                    restore_bounds,
                    cascade_offset,
                    center_in_work_area,
                    geometry,
                    resizable,
                    content_chrome,
                    close_request,
                    focus,
                    blur,
                    appearance,
                },
                window_state.clone(),
                runtime.clone(),
                cx,
            )
            .map(|(handle, view)| {
                windows.insert(
                    key,
                    ManagedWindow {
                        handle,
                        view,
                        state: window_state,
                        runtime,
                    },
                );
            });
            send_reply(reply, result);
        }
        WindowCommand::Update {
            runtime_id,
            window_id,
            tree,
            reply,
        } => {
            let result = update_gpui_window(windows, (runtime_id, window_id), *tree, cx);
            send_reply(reply, result);
        }
        WindowCommand::Close {
            runtime_id,
            window_id,
            reply,
        } => {
            let result = close_gpui_window(windows, (runtime_id, window_id), cx);
            send_reply(reply, result);
        }
        WindowCommand::AwaitFrame {
            runtime_id,
            window_id,
            reply,
        } => await_gpui_frame(windows, (runtime_id, window_id), reply, cx),
        WindowCommand::FrameToken {
            runtime_id,
            window_id,
            reply,
        } => gpui_frame_token(windows, (runtime_id, window_id), reply),
        WindowCommand::AwaitFrameAfter {
            runtime_id,
            window_id,
            generation,
            reply,
        } => await_gpui_frame_after(windows, (runtime_id, window_id), generation, reply),
        WindowCommand::ShutdownRuntime { runtime_id, reply } => {
            let keys = windows
                .keys()
                .filter(|(owner_id, _window_id)| *owner_id == runtime_id)
                .copied()
                .collect::<Vec<_>>();

            let result = keys
                .into_iter()
                .try_for_each(|key| close_gpui_window(windows, key, cx));

            if let Some(reply) = reply {
                send_reply(reply, result);
            }
        }
        WindowCommand::ShowSystemNotification {
            tag,
            title,
            subtitle,
            body,
            sound,
            reply,
        } => {
            cx.show_system_notification_with_options(
                gpui::SystemNotification {
                    tag: tag.into(),
                    title: title.into(),
                    body: body.into(),
                    actions: Vec::new(),
                },
                gpui::SystemNotificationOptions {
                    subtitle: subtitle.into(),
                    sound,
                },
            );
            send_reply(reply, Ok(()));
        }
        WindowCommand::DismissSystemNotification { tag, reply } => {
            cx.dismiss_system_notification(&tag);
            send_reply(reply, Ok(()));
        }
        WindowCommand::SubscribeSystemNotifications { pid, reply } => {
            cx.on_system_notification_response(move |response, _cx| {
                let mut env = rustler::OwnedEnv::new();
                let _ = env.send_and_clear(&pid, |env| {
                    let marker = rustler::Atom::from_str(env, "gpui_system_notification").unwrap();
                    (
                        marker,
                        response.tag.to_string(),
                        response.action_id.map(|id| id.to_string()),
                    )
                        .encode(env)
                });
            });
            send_reply(reply, Ok(()));
        }
        WindowCommand::SystemNotificationPermission { request, reply } => {
            let task = if request {
                cx.request_system_notifications()
            } else {
                cx.system_notification_status()
            };
            cx.spawn(async move |_| {
                let result = task
                    .await
                    .map(|status| {
                        match status {
                            gpui::SystemNotificationStatus::Authorized => "authorized",
                            gpui::SystemNotificationStatus::Denied => "denied",
                            gpui::SystemNotificationStatus::NotDetermined => "not_determined",
                            gpui::SystemNotificationStatus::Unsupported => "unsupported",
                        }
                        .to_string()
                    })
                    .map_err(|error| error.to_string());
                let _ = reply.send(result);
            })
            .detach();
        }
        WindowCommand::ActivateWindow {
            runtime_id,
            window_id,
            reply,
        } => {
            let result = windows
                .get(&(runtime_id, window_id))
                .ok_or_else(|| "unknown_window".to_string())
                .and_then(|managed| {
                    managed
                        .handle
                        .update(cx, |_root, window, cx| {
                            window.activate_window();
                            cx.activate(true);
                        })
                        .map_err(|error| error.to_string())
                });
            send_reply(reply, result);
        }
        WindowCommand::SetAppIdentity {
            identifier,
            name,
            reply,
        } => {
            let requested = (identifier, name);
            let result = match identity {
                Some(existing) if existing != &requested => {
                    Err("application_identity_conflict".to_string())
                }
                Some(_existing) => Ok(()),
                None => {
                    cx.set_app_identity(&requested.0, &requested.1);
                    *identity = Some(requested);
                    Ok(())
                }
            };
            send_reply(reply, result);
        }
        WindowCommand::LoadFont { bytes, reply } => {
            let result = cx
                .text_system()
                .add_fonts(vec![std::borrow::Cow::Owned(bytes)])
                .map_err(|error| error.to_string());
            send_reply(reply, result);
        }
        WindowCommand::SetTheme { mode, reply } => {
            let result = set_component_theme(mode, windows, cx);
            send_reply(reply, result);
        }
        WindowCommand::PlatformClosed { platform_id } => {
            handle_platform_window_closed(windows, platform_id);
        }
    }
}

#[cfg(feature = "components")]
fn set_component_theme(
    mode: NativeThemeMode,
    windows: &mut HashMap<WindowKey, ManagedWindow>,
    cx: &mut gpui::App,
) -> Result<(), String> {
    match mode {
        NativeThemeMode::System => {
            cx.set_window_appearance(None);
            gpui_component::Theme::sync_system_appearance(None, cx);
        }
        NativeThemeMode::Light => {
            cx.set_window_appearance(Some(gpui::WindowAppearance::Light));
            gpui_component::Theme::change(gpui_component::ThemeMode::Light, None, cx);
        }
        NativeThemeMode::Dark => {
            cx.set_window_appearance(Some(gpui::WindowAppearance::Dark));
            gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
        }
    }

    windows.values().try_for_each(|window| {
        window
            .handle
            .update(cx, |_root, native_window, _cx| native_window.refresh())
            .map_err(|error| error.to_string())
    })
}

#[cfg(all(feature = "real-gpui", not(feature = "components")))]
fn set_component_theme(
    _mode: NativeThemeMode,
    _windows: &mut HashMap<WindowKey, ManagedWindow>,
    _cx: &mut gpui::App,
) -> Result<(), String> {
    Err("components_disabled".to_string())
}

#[cfg(feature = "real-gpui")]
fn handle_platform_window_closed(
    windows: &mut HashMap<WindowKey, ManagedWindow>,
    platform_id: gpui::WindowId,
) {
    let key = windows
        .iter()
        .find_map(|(key, window)| (window.handle.window_id() == platform_id).then_some(*key));

    if let Some(key) = key {
        if let Some(window) = windows.remove(&key) {
            if let Ok(mut bounds) = window.runtime.element_bounds.lock() {
                bounds.forget_window(key.1);
            }
            let _ = push_event(
                &window.runtime,
                NativeEvent::WindowClosed { window_id: key.1 },
            );
        }
    }
}

#[cfg(feature = "real-gpui")]
fn send_reply(reply: WindowCommandReply, result: Result<(), String>) {
    let _ = reply.send(result);
}

#[cfg(feature = "real-gpui")]
fn update_gpui_window(
    windows: &mut HashMap<WindowKey, ManagedWindow>,
    key: WindowKey,
    tree: ElementNode,
    cx: &mut gpui::App,
) -> Result<(), String> {
    let window = windows
        .get(&key)
        .ok_or_else(|| "unknown_window".to_string())?;
    *window
        .state
        .tree
        .lock()
        .map_err(|_| "runtime_lock_failed".to_string())? = tree;
    window
        .state
        .requested_generation
        .fetch_add(1, std::sync::atomic::Ordering::AcqRel);

    let view = window.view.clone();

    let result = window.handle.update(cx, |_root, native_window, cx| {
        native_window.defer(cx, move |native_window, cx| {
            view.update(cx, |_view, cx| cx.notify());
            native_window.refresh();
            cx.refresh_windows();
        });
    });
    result.map_err(|error| error.to_string())
}

#[cfg(feature = "real-gpui")]
fn await_gpui_frame(
    windows: &HashMap<WindowKey, ManagedWindow>,
    key: WindowKey,
    reply: WindowCommandReply,
    _cx: &mut gpui::App,
) {
    use std::sync::atomic::Ordering;

    let Some(window) = windows.get(&key) else {
        send_reply(reply, Err("unknown_window".to_string()));
        return;
    };

    let target = window.state.requested_generation.load(Ordering::Acquire);
    await_generation(
        &window.state.rendered_generation,
        &window.state.frame_waiters,
        target,
        reply,
    );
}

#[cfg(feature = "real-gpui")]
fn gpui_frame_token(
    windows: &HashMap<WindowKey, ManagedWindow>,
    key: WindowKey,
    reply: WindowGenerationReply,
) {
    use std::sync::atomic::Ordering;

    let result = windows
        .get(&key)
        .map(|window| {
            window
                .state
                .completed_frame_generation
                .load(Ordering::Acquire)
        })
        .ok_or_else(|| "unknown_window".to_string());
    let _ = reply.send(result);
}

#[cfg(feature = "real-gpui")]
fn await_gpui_frame_after(
    windows: &HashMap<WindowKey, ManagedWindow>,
    key: WindowKey,
    generation: u64,
    reply: WindowCommandReply,
) {
    let Some(window) = windows.get(&key) else {
        send_reply(reply, Err("unknown_window".to_string()));
        return;
    };

    await_generation(
        &window.state.completed_frame_generation,
        &window.state.next_frame_waiters,
        generation.saturating_add(1),
        reply,
    );
}

#[cfg(feature = "real-gpui")]
fn close_gpui_window(
    windows: &mut HashMap<WindowKey, ManagedWindow>,
    key: WindowKey,
    cx: &mut gpui::App,
) -> Result<(), String> {
    let window = windows
        .remove(&key)
        .ok_or_else(|| "unknown_window".to_string())?;
    let runtime = window.runtime.clone();
    close_managed_window(window, cx)?;
    if let Ok(mut bounds) = runtime.element_bounds.lock() {
        bounds.forget_window(key.1);
    }
    push_event(&runtime, NativeEvent::WindowClosed { window_id: key.1 })
        .map_err(|_error| "runtime_lock_failed".to_string())
}

#[cfg(feature = "real-gpui")]
fn close_managed_window(window: ManagedWindow, cx: &mut gpui::App) -> Result<(), String> {
    window
        .handle
        .update(cx, |_root, native_window, _cx| {
            native_window.remove_window()
        })
        .map_err(|error| error.to_string())
}

#[cfg(feature = "real-gpui")]
struct WindowOpenConfig {
    title: String,
    window_id: u64,
    width: f32,
    height: f32,
    min_size: Option<(f32, f32)>,
    restore_bounds: Option<Vec<f64>>,
    cascade_offset: u32,
    center_in_work_area: bool,
    geometry: bool,
    resizable: bool,
    content_chrome: bool,
    close_request: bool,
    focus: bool,
    blur: bool,
    appearance: bool,
}

#[cfg(feature = "real-gpui")]
fn open_gpui_window(
    config: WindowOpenConfig,
    window_state: SharedWindow,
    runtime: SharedRuntime,
    cx: &mut gpui::App,
) -> Result<
    (
        gpui::WindowHandle<NativeWindowRoot>,
        gpui::Entity<ElixirRoot>,
    ),
    String,
> {
    use gpui::{
        px, size, AppContext, Bounds, TitlebarOptions, WindowBounds, WindowDecorations,
        WindowOptions,
    };

    let WindowOpenConfig {
        title,
        window_id,
        width,
        height,
        min_size,
        restore_bounds,
        cascade_offset,
        center_in_work_area,
        geometry,
        resizable,
        content_chrome,
        close_request,
        focus,
        blur,
        appearance,
    } = config;
    let centered = if center_in_work_area {
        cx.primary_display()
            .map(|display| {
                work_area_center(
                    display.visible_bounds(),
                    size(px(width), px(height)),
                    display.scale_factor(),
                )
            })
            .unwrap_or_else(|| Bounds::centered(None, size(px(width), px(height)), cx))
    } else {
        Bounds::centered(None, size(px(width), px(height)), cx)
    };
    let restored = restored_window_bounds(restore_bounds.as_deref(), cascade_offset, cx);
    let bounds = restored.unwrap_or(centered);
    let window_min_size = min_size.map(|(width, height)| size(px(width), px(height)));
    let titlebar = content_chrome.then(|| TitlebarOptions {
        title: Some(title.clone().into()),
        appears_transparent: true,
        traffic_light_position: Some(gpui::point(px(9.0), px(9.0))),
    });
    let titlebar = if content_chrome {
        titlebar
    } else {
        Some(TitlebarOptions {
            title: Some(title.clone().into()),
            ..Default::default()
        })
    };
    let window_decorations = content_chrome.then_some(WindowDecorations::Client);
    let view = cx.new(|_cx| {
        let mut root = ElixirRoot::new(window_state, runtime.clone(), window_id, focus, blur);
        root.appearance_window = appearance;
        root.geometry_window = geometry;
        root
    });
    let view_for_root = view.clone();

    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                is_resizable: resizable,
                window_min_size,
                titlebar,
                app_owns_titlebar_drag: content_chrome,
                window_decorations,
                ..Default::default()
            },
            |native_window, cx| {
                native_window.set_window_title(&title);
                if close_request {
                    let runtime = runtime.clone();
                    native_window.on_window_should_close(cx, move |_window, _cx| {
                        let _ = push_event(&runtime, NativeEvent::WindowCloseRequest { window_id });
                        false
                    });
                }
                native_window_root(view_for_root.clone(), native_window, cx)
            },
        )
        .map_err(|error| error.to_string())?;

    Ok((handle, view))
}

#[cfg(feature = "real-gpui")]
fn work_area_center(
    area: gpui::Bounds<gpui::Pixels>,
    size: gpui::Size<gpui::Pixels>,
    scale: f32,
) -> gpui::Bounds<gpui::Pixels> {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let x = ((f32::from(area.size.width) - f32::from(size.width)) * scale / 2.0).trunc() / scale;
    let y = ((f32::from(area.size.height) - f32::from(size.height)) * scale / 2.0).trunc() / scale;
    gpui::Bounds::new(area.origin + gpui::point(gpui::px(x), gpui::px(y)), size)
}

#[cfg(feature = "real-gpui")]
fn restored_window_bounds(
    saved: Option<&[f64]>,
    offset: u32,
    cx: &gpui::App,
) -> Option<gpui::Bounds<gpui::Pixels>> {
    let [x, y, width, height] = saved? else {
        return None;
    };
    if ![x, y, width, height].iter().all(|value| value.is_finite())
        || *width <= 0.0
        || *height <= 0.0
    {
        return None;
    }
    let frame = gpui::Bounds::new(
        gpui::point(gpui::px(*x as f32), gpui::px(*y as f32)),
        gpui::size(gpui::px(*width as f32), gpui::px(*height as f32)),
    );
    let area = cx
        .displays()
        .into_iter()
        .map(|display| display.visible_bounds())
        .find(|area| {
            let slack = gpui::px(1.0);
            frame.origin.x >= area.origin.x - slack
                && frame.origin.y >= area.origin.y - slack
                && frame.right() <= area.right() + slack
                && frame.bottom() <= area.bottom() + slack
        })?;
    let mut result = frame;
    let offset = gpui::px(offset as f32);
    result.origin.x += offset;
    result.origin.y += offset;
    if offset > gpui::px(0.0) && (result.right() > area.right() || result.bottom() > area.bottom())
    {
        result.origin = area.origin + gpui::point(offset, offset);
    }
    Some(result)
}

#[cfg(feature = "components")]
fn native_window_root(
    view: gpui::Entity<ElixirRoot>,
    native_window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> gpui::Entity<NativeWindowRoot> {
    use gpui::AppContext;

    cx.new(|cx| gpui_component::Root::new(view, native_window, cx))
}

#[cfg(not(feature = "components"))]
fn native_window_root(
    view: gpui::Entity<ElixirRoot>,
    _native_window: &mut gpui::Window,
    _cx: &mut gpui::App,
) -> gpui::Entity<NativeWindowRoot> {
    view
}

#[cfg(all(test, feature = "real-gpui"))]
mod appearance_tests {
    use super::*;

    #[test]
    fn work_area_center_matches_physical_pixel_integer_division() {
        let area = gpui::Bounds::new(
            gpui::point(gpui::px(-1784.0), gpui::px(30.0)),
            gpui::size(gpui::px(1784.0), gpui::px(1015.0)),
        );
        let size = gpui::size(gpui::px(1400.0), gpui::px(900.0));
        assert_eq!(
            work_area_center(area, size, 1.0).origin,
            gpui::point(gpui::px(-1592.0), gpui::px(87.0))
        );
        assert_eq!(
            work_area_center(area, size, 2.0).origin,
            gpui::point(gpui::px(-1592.0), gpui::px(87.5))
        );
        let oversized = gpui::size(gpui::px(1800.0), gpui::px(1020.0));
        assert_eq!(
            work_area_center(area, oversized, 1.0).origin.y,
            gpui::px(28.0)
        );
    }

    #[gpui::test]
    fn restored_bounds_validate_displays_and_cascade(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let area = cx.displays()[0].visible_bounds();
            let x = f32::from(area.origin.x) as f64 + 50.0;
            let y = f32::from(area.origin.y) as f64 + 50.0;
            let saved = [x, y, 400.0, 300.0];
            let restored = restored_window_bounds(Some(&saved), 0, cx).unwrap();
            assert_eq!(
                restored.origin,
                gpui::point(gpui::px(x as f32), gpui::px(y as f32))
            );
            let cascaded = restored_window_bounds(Some(&saved), 36, cx).unwrap();
            assert_eq!(
                cascaded.origin,
                restored.origin + gpui::point(gpui::px(36.0), gpui::px(36.0))
            );
            let edge = [
                f32::from(area.right()) as f64 - 400.0,
                f32::from(area.bottom()) as f64 - 300.0,
                400.0,
                300.0,
            ];
            let rounded_edge = [edge[0] + 0.5, edge[1] + 0.5, edge[2], edge[3]];
            let rounded = restored_window_bounds(Some(&rounded_edge), 0, cx).unwrap();
            assert_eq!(f32::from(rounded.origin.x), rounded_edge[0] as f32);
            assert_eq!(f32::from(rounded.origin.y), rounded_edge[1] as f32);
            let wrapped = restored_window_bounds(Some(&edge), 36, cx).unwrap();
            assert_eq!(
                wrapped.origin,
                area.origin + gpui::point(gpui::px(36.0), gpui::px(36.0))
            );
            assert!(
                restored_window_bounds(Some(&[100000.0, 100000.0, 400.0, 300.0]), 0, cx).is_none()
            );
            assert!(restored_window_bounds(Some(&[0.0, 0.0, f64::NAN, 300.0]), 0, cx).is_none());
        });
    }

    #[gpui::test]
    fn geometry_observer_reports_initial_bounds_once(cx: &mut gpui::TestAppContext) {
        let runtime = Arc::new(crate::runtime::RuntimeState::new());
        let state = Arc::new(WindowState::new(
            ElementNode::Text(crate::TextNode {
                truncate_start: false,
                text: "Geometry".into(),
                style: Default::default(),
            }),
            Vec::new(),
        ));
        let owner = runtime.clone();
        let _window = cx.add_window(move |window, cx| {
            let mut root = ElixirRoot::new(state, owner, 10, false, false);
            root.geometry_window = true;
            root.observe_geometry(window, cx);
            root.observe_geometry(window, cx);
            root.report_geometry(window, false);
            root
        });
        cx.run_until_parked();
        let events = runtime.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0], NativeEvent::WindowGeometry {window_id: 10, initial: true, bounds} if bounds.len() == 4)
        );
    }

    #[gpui::test]
    fn appearance_observer_reports_initial_mode_without_component_initialization(
        cx: &mut gpui::TestAppContext,
    ) {
        let runtime = Arc::new(crate::runtime::RuntimeState::new());
        let state = Arc::new(WindowState::new(
            ElementNode::Text(crate::TextNode {
                truncate_start: false,
                text: "Appearance".into(),
                style: Default::default(),
            }),
            Vec::new(),
        ));
        let runtime_for_view = runtime.clone();
        let _window = cx.add_window(move |window, cx| {
            let mut root = ElixirRoot::new(state, runtime_for_view, 9, false, false);
            root.appearance_window = true;
            root.observe_appearance(window, cx);
            root
        });
        cx.run_until_parked();
        let events = runtime.events.lock().expect("event queue");
        assert!(matches!(
            events.first(),
            Some(NativeEvent::WindowAppearance {
                window_id: 9,
                dark: false,
            })
        ));
    }
}
