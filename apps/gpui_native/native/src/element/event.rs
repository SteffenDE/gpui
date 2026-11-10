use crate::*;

#[cfg(all(feature = "real-gpui", feature = "components"))]
pub(crate) fn attach_accessible_action_target(
    element: gpui::Stateful<gpui::Div>,
    test_id: String,
    event: String,
    accessibility: super::AccessibilitySemantics,
    runtime: SharedRuntime,
    window_id: u64,
    focus: gpui::FocusHandle,
) -> gpui::Stateful<gpui::Div> {
    use gpui::{AccessibleAction, InteractiveElement, StatefulInteractiveElement};

    let enabled = !accessibility.disabled;
    let mut element = super::apply_accessibility_semantics(element, accessibility);
    element = element.debug_selector(|| test_id.clone());
    let focus = focus.tab_stop(enabled);
    if enabled {
        if let Ok(mut handles) = runtime.focus_handles.lock() {
            handles.insert((window_id, test_id), focus.clone());
        }
        element = element.track_focus(&focus);

        let keyboard_runtime = runtime.clone();
        let keyboard_event = event.clone();
        element = element
            .key_context("GPUIAccessibleControl")
            .tab_index(0)
            .focus_visible(|style| style.border_2().border_color(gpui::rgb(0x60a5fa)))
            .on_key_down(move |key, window, cx| {
                if matches!(key.keystroke.key.as_str(), "enter" | "space") {
                    window.prevent_default();
                    emit_click_event(&keyboard_runtime, window_id, &keyboard_event);
                    cx.stop_propagation();
                }
            })
            .on_a11y_action(AccessibleAction::Click, move |_data, _window, _cx| {
                emit_click_event(&runtime, window_id, &event);
            });
    }

    element
}

#[cfg(feature = "real-gpui")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_click_event(
    element: gpui::Div,
    element_id: String,
    event: Option<String>,
    accessibility: super::AccessibilitySemantics,
    runtime: SharedRuntime,
    window_id: u64,
    motion: super::ContainerMotion,
    tab_stop: bool,
    window_control: Option<String>,
    clipboard: (Option<String>, Option<String>, bool),
    captures_pointer: bool,
    focus_border: bool,
) -> gpui::AnyElement {
    use gpui::{AccessibleAction, InteractiveElement, StatefulInteractiveElement};

    let debug_selector = element_id.clone();
    let element_id = if event.is_some() {
        format!("gpui-elixir-click-{window_id}-{element_id}")
    } else {
        element_id
    };
    let element = element.debug_selector(|| debug_selector).id(element_id);
    let element = super::apply_window_control(element, window_control, runtime.clone(), window_id);
    if event.is_some() || clipboard.0.is_some() {
        let enabled = !accessibility.disabled;
        let activatable = enabled
            && accessibility
                .role
                .as_ref()
                .is_some_and(AccessibilityRole::is_activatable);
        let element = super::apply_accessibility_semantics(element, accessibility.clone());
        let element = if activatable {
            // GPUI synthesizes one click on an unmodified Enter/Space release.
            // An extra key-down handler here would activate controls twice.
            let element = element
                .key_context("GPUIAccessibleControl")
                .tab_index(0)
                .tab_stop(tab_stop);
            if focus_border {
                element.focus_visible(|style| style.border_2().border_color(gpui::rgb(0x60a5fa)))
            } else {
                element
            }
        } else {
            element
        };
        let pointer_runtime = runtime.clone();
        let pointer_event = event.clone();
        let pointer_clipboard = clipboard.clone();

        let element = if enabled {
            element
                // Like phx-click in LiveView, only the innermost clickable
                // element under the pointer handles the click.
                .on_click(move |event, _window, cx| {
                    if captures_pointer && matches!(event, gpui::ClickEvent::Mouse(_)) {
                        cx.stop_propagation();
                        return;
                    }
                    perform_clipboard_action(
                        &pointer_runtime,
                        window_id,
                        &pointer_clipboard,
                        event.modifiers(),
                        cx,
                    );
                    if let Some(name) = &pointer_event {
                        emit_pointer_click(&pointer_runtime, window_id, name, event.modifiers());
                    }
                    cx.stop_propagation();
                })
                .on_a11y_action(AccessibleAction::Click, move |_data, _window, cx| {
                    perform_clipboard_action(
                        &runtime,
                        window_id,
                        &clipboard,
                        gpui::Modifiers::default(),
                        cx,
                    );
                    if let Some(event) = &event {
                        emit_click_event(&runtime, window_id, event);
                    }
                })
        } else {
            element
        };

        let actions = if enabled {
            vec![AccessibleAction::Click]
        } else {
            vec![]
        };
        super::apply_container_motion(element, motion, accessibility, actions)
    } else {
        super::apply_container_motion(element, motion, accessibility, vec![])
    }
}

#[cfg(feature = "real-gpui")]
pub(crate) fn emit_pointer_position(
    runtime: &SharedRuntime,
    window_id: u64,
    event: &str,
    position: gpui::Point<gpui::Pixels>,
) {
    let _ = push_event(
        runtime,
        NativeEvent::Input {
            kind: InputKind::Change,
            window_id,
            event: event.to_string(),
            value: Some(EventValue::Numbers(vec![
                f64::from(f32::from(position.x)),
                f64::from(f32::from(position.y)),
            ])),
        },
    );
}

#[cfg(feature = "real-gpui")]
pub(crate) fn emit_pointer_click(
    runtime: &SharedRuntime,
    window_id: u64,
    event: &str,
    modifiers: gpui::Modifiers,
) {
    if modifiers.control || modifiers.alt || modifiers.shift || modifiers.platform {
        let _ = push_event(
            runtime,
            NativeEvent::ModifiedClick {
                window_id,
                event: event.to_string(),
                control: modifiers.control,
                alt: modifiers.alt,
                shift: modifiers.shift,
                platform: modifiers.platform,
            },
        );
    } else {
        emit_click_event(runtime, window_id, event);
    }
}

#[cfg(feature = "real-gpui")]
pub(crate) fn emit_click_event(runtime: &SharedRuntime, window_id: u64, event: &str) {
    let _ = push_event(
        runtime,
        NativeEvent::Click {
            window_id,
            event: event.to_string(),
        },
    );
}

#[cfg(feature = "real-gpui")]
pub(crate) fn apply_input_events(
    element: gpui::Div,
    input_id: String,
    keydown: Option<String>,
    keyup: Option<String>,
    structured: (Option<String>, Option<String>),
    runtime: SharedRuntime,
    window_id: u64,
) -> gpui::AnyElement {
    use gpui::{InteractiveElement, IntoElement};

    let mut element = element.key_context("GPUIInput").tab_index(0).id(input_id);

    if let Some(event) = keydown {
        let runtime = runtime.clone();
        element = element.on_key_down(move |key_event, _window, _cx| {
            let value = key_event
                .keystroke
                .key_char
                .clone()
                .or_else(|| Some(key_event.keystroke.key.clone()))
                .map(EventValue::String);

            let _ = push_event(
                &runtime,
                NativeEvent::Input {
                    kind: InputKind::KeyDown,
                    window_id,
                    event: event.clone(),
                    value,
                },
            );
        });
    }

    if let Some(event) = keyup {
        let runtime = runtime.clone();
        element = element.on_key_up(move |key_event, _window, _cx| {
            let value = key_event
                .keystroke
                .key_char
                .clone()
                .or_else(|| Some(key_event.keystroke.key.clone()))
                .map(EventValue::String);

            let _ = push_event(
                &runtime,
                NativeEvent::Input {
                    kind: InputKind::KeyUp,
                    window_id,
                    event: event.clone(),
                    value,
                },
            );
        });
    }

    if let Some(event) = structured.0 {
        let runtime = runtime.clone();
        element = element.on_key_down(move |key, _, _| {
            emit_structured_key(&runtime, window_id, &event, &key.keystroke, key.is_held);
        });
    }
    if let Some(event) = structured.1 {
        element = element.on_key_up(move |key, _, _| {
            emit_structured_key(&runtime, window_id, &event, &key.keystroke, false);
        });
    }

    element.into_any_element()
}

#[cfg(feature = "real-gpui")]
pub(crate) fn emit_structured_key(
    runtime: &SharedRuntime,
    window_id: u64,
    event: &str,
    key: &gpui::Keystroke,
    repeat: bool,
) {
    let mods = key.modifiers;
    let _ = push_event(
        runtime,
        NativeEvent::Keystroke {
            window_id,
            event: event.to_string(),
            value: crate::event::KeyStroke {
                key: key.key.clone(),
                control: mods.control,
                alt: mods.alt,
                shift: mods.shift,
                platform: mods.platform,
                repeat,
            },
        },
    );
}

#[cfg(all(test, feature = "real-gpui"))]
mod tests {
    use super::emit_click_event;
    use crate::gpui;
    use crate::{event::NativeEvent, runtime::RuntimeState, AccessibilityRole};
    use std::sync::Arc;

    #[test]
    fn accessible_action_attachment_uses_the_shared_click_event_path() {
        let runtime = Arc::new(RuntimeState::new());
        emit_click_event(&runtime, 9, "open-settings");

        let events = runtime.events.lock().expect("event queue");
        assert!(matches!(
            events.as_slice(),
            [NativeEvent::Click { window_id: 9, event }] if event == "open-settings"
        ));
    }

    #[test]
    fn generated_role_policy_only_tab_stops_simple_activation_controls() {
        for role in [
            AccessibilityRole::Button,
            AccessibilityRole::Checkbox,
            AccessibilityRole::Link,
            AccessibilityRole::MenuItem,
            AccessibilityRole::Radio,
            AccessibilityRole::Switch,
        ] {
            assert!(role.is_activatable());
        }

        for role in [
            AccessibilityRole::Group,
            AccessibilityRole::List,
            AccessibilityRole::Menu,
            AccessibilityRole::TabList,
            AccessibilityRole::Tree,
            AccessibilityRole::Slider,
        ] {
            assert!(!role.is_activatable());
        }
    }

    fn clickable(id: &str, width: f32, children: Vec<crate::ElementNode>) -> crate::ElementNode {
        let mut style = crate::StyleAttrs::default();
        style.width = Some(gpui::px(width).into());
        style.height = Some(gpui::px(20.0).into());
        crate::ElementNode::Div(crate::ContainerNode {
            tag: crate::GeneratedElementTag::Div,
            style,
            id: Some(id.to_string()),
            accessibility: crate::AccessibilitySemantics::default(),
            children,
            pointer: None,
            pointer_position: None,
            pointer_down: None,
            window_pointer_position: None,
            window_pointer_up: None,
            clipboard_write: None,
            clipboard_text: None,
            clipboard_write_request: 0,
            clipboard_without_primary: false,
            click: Some(id.to_string()),
            bounds_change: None,
            scrollbar: false,
            scroll_top: 0.0,
            scroll_left: 0.0,
            scroll_horizontal: false,
            scroll_request: 0,
            scroll_event: None,
            keycapture: None,
            window_keyrelease: None,
            capture_keys: Vec::new(),
            keyrelease: None,
            focus_request: 0,
            tab_stop: true,
            focus: None,
            blur: None,
            motion_request: 0,
            motion_duration: 180,
            motion_delay: 0,
            motion_easing: "ease_out".to_string(),
            motion_policy: "respect_system".to_string(),
            motion_from_opacity: 1.0,
            motion_from_x: 0.0,
            motion_from_y: 0.0,
            motion_to_opacity: 1.0,
            motion_to_x: 0.0,
            motion_to_y: 0.0,
            motion_repeat: false,
            motion_alternate: false,
            window_control: None,
        })
    }

    #[gpui::test]
    fn only_the_innermost_clickable_element_handles_a_click(cx: &mut gpui::TestAppContext) {
        use gpui::{point, px, size, IntoElement, Modifiers};

        let tree = clickable("outer", 200.0, vec![clickable("inner", 50.0, Vec::new())]);
        let runtime = Arc::new(RuntimeState::new());
        let state = Arc::new(crate::WindowState::new(tree, Vec::new()));
        let runtime_for_view = runtime.clone();
        let (view, cx) = cx.add_window_view(move |_window, _cx| {
            crate::ElixirRoot::new(state, runtime_for_view, 7, false, false)
        });
        cx.draw(
            point(px(0.0), px(0.0)),
            size(px(300.0), px(100.0)),
            |_window, _cx| view.clone().into_any_element(),
        );

        cx.simulate_click(point(px(10.0), px(10.0)), Modifiers::default());
        cx.simulate_click(point(px(100.0), px(10.0)), Modifiers::default());

        let clicked = runtime
            .events
            .lock()
            .expect("event queue")
            .iter()
            .filter_map(|event| match event {
                NativeEvent::Click { event, .. } => Some(event.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(clicked, vec!["inner", "outer"]);
        cx.simulate_click(
            point(px(10.0), px(10.0)),
            Modifiers {
                platform: true,
                shift: true,
                ..Modifiers::default()
            },
        );
        assert!(matches!(runtime.events.lock().expect("event queue").last(),
            Some(NativeEvent::ModifiedClick {window_id: 7, event, platform: true, shift: true, control: false, alt: false})
            if event == "inner"));
    }

    #[test]
    fn pointer_and_accessible_activation_share_the_same_native_event_path() {
        let runtime = Arc::new(RuntimeState::new());

        emit_click_event(&runtime, 7, "toggle");
        emit_click_event(&runtime, 7, "toggle");

        let events = runtime.events.lock().expect("event queue");
        assert_eq!(events.len(), 2);
        for event in events.iter() {
            assert!(matches!(
                event,
                NativeEvent::Click { window_id: 7, event } if event == "toggle"
            ));
        }
    }

    #[gpui::test]
    fn pointer_state_survives_render_and_primary_click_does_not_copy(
        cx: &mut gpui::TestAppContext,
    ) {
        use gpui::{point, px, size, IntoElement, Modifiers};
        let mut tree = clickable("name", 200.0, Vec::new());
        if let crate::ElementNode::Div(node) = &mut tree {
            node.pointer = Some("hover".to_string());
            node.clipboard_write = Some("copied".to_string());
            node.clipboard_text = Some("folder/file.ex".to_string());
            node.clipboard_without_primary = true;
        }
        let runtime = Arc::new(RuntimeState::new());
        let state = Arc::new(crate::WindowState::new(tree, Vec::new()));
        let for_view = runtime.clone();
        let (view, cx) = cx
            .add_window_view(move |_, _| crate::ElixirRoot::new(state, for_view, 9, false, false));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(
                point(px(0.0), px(0.0)),
                size(px(300.0), px(100.0)),
                |_, _| view.clone().into_any_element(),
            );
        };
        draw(cx);
        cx.simulate_mouse_move(point(px(10.0), px(10.0)), None, Modifiers::default());
        assert!(runtime.events.lock().unwrap().iter().any(|event| matches!(
            event,
            NativeEvent::PointerState {
                hovered: true,
                platform: false,
                ..
            }
        )));
        draw(cx);
        let modified = Modifiers {
            platform: true,
            control: true,
            shift: true,
            ..Modifiers::default()
        };
        cx.simulate_modifiers_change(modified);
        assert!(matches!(
            runtime.events.lock().unwrap().last(),
            Some(NativeEvent::PointerState {
                hovered: true,
                platform: true,
                shift: true,
                ..
            })
        ));
        cx.simulate_click(point(px(10.0), px(10.0)), Modifiers::default());
        cx.simulate_click(point(px(10.0), px(10.0)), modified);
        let copies = runtime
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| matches!(event, NativeEvent::PrimitiveClipboardWrite { .. }))
            .count();
        assert_eq!(copies, 1);
        cx.simulate_mouse_move(point(px(250.0), px(10.0)), None, Modifiers::default());
        assert!(matches!(
            runtime.events.lock().unwrap().last(),
            Some(NativeEvent::PointerState { hovered: false, .. })
        ));
    }
}

#[cfg(feature = "real-gpui")]
pub(crate) fn apply_pointer_observer(
    element: gpui::Div,
    id: String,
    event: String,
    context: &mut super::ElementRenderContext<'_, '_>,
) -> gpui::Div {
    use gpui::{ParentElement, Styled};
    let hovered = context.window.use_keyed_state(
        format!("gpui-pointer-{}-{id}", context.id_namespace),
        context.cx,
        |_, _| false,
    );
    let runtime = context.runtime.clone();
    let window_id = context.window_id;
    let state = hovered.clone();
    let observed_event = event.clone();
    let observed_runtime = runtime.clone();
    let update = std::rc::Rc::new(
        move |entered: bool, window: &mut gpui::Window, cx: &mut gpui::App| {
            let changed = state.update(cx, |previous, _| {
                let changed = *previous != entered;
                *previous = entered;
                changed
            });
            if changed {
                emit_pointer_state(
                    &observed_runtime,
                    window_id,
                    &observed_event,
                    entered,
                    window.modifiers(),
                );
            }
        },
    );
    let frame_state = hovered.clone();
    let observer = gpui::canvas(
        |bounds, window, _cx| {
            window.insert_hitbox(
                bounds.intersect(&window.content_mask().bounds),
                gpui::HitboxBehavior::Normal,
            )
        },
        move |_bounds, hitbox, window, cx| {
            // GPUI's default hover listener clears on keyboard input. These observations
            // describe physical pointer presence and must survive a stationary key press.
            let entered = if window.last_input_was_keyboard() {
                *frame_state.read(cx) && hitbox.bounds.contains(&window.mouse_position())
            } else {
                hitbox.is_hovered(window)
            };
            let frame_update = update.clone();
            window.defer(cx, move |window, cx| frame_update(entered, window, cx));
            let move_update = update.clone();
            let move_hitbox = hitbox.clone();
            window.on_mouse_event(move |_: &gpui::MouseMoveEvent, phase, window, cx| {
                if phase == gpui::DispatchPhase::Bubble {
                    move_update(move_hitbox.is_hovered(window), window, cx);
                }
            });
            let exit_update = update.clone();
            window.on_mouse_event(move |_: &gpui::MouseExitEvent, phase, window, cx| {
                if phase == gpui::DispatchPhase::Bubble {
                    exit_update(false, window, cx);
                }
            });
        },
    )
    .absolute()
    .inset_0();
    context.pointer_observers.borrow_mut().insert(
        format!("{}:{id}", context.id_namespace),
        (runtime, window_id, event, hovered),
    );
    element.child(observer)
}

#[cfg(feature = "real-gpui")]
fn emit_pointer_state(
    runtime: &SharedRuntime,
    window_id: u64,
    event: &str,
    hovered: bool,
    modifiers: gpui::Modifiers,
) {
    let _ = push_event(
        runtime,
        NativeEvent::PointerState {
            window_id,
            event: event.to_string(),
            hovered,
            control: modifiers.control,
            alt: modifiers.alt,
            shift: modifiers.shift,
            platform: modifiers.platform,
        },
    );
}

#[cfg(feature = "real-gpui")]
pub(crate) fn apply_clipboard_request(
    id: &str,
    request: u64,
    clipboard: &(Option<String>, Option<String>, bool),
    context: &mut super::ElementRenderContext<'_, '_>,
) {
    if clipboard.0.is_none() || clipboard.1.is_none() {
        return;
    }
    let key = (context.window_id, format!("{}-{id}", context.id_namespace));
    let changed = context
        .runtime
        .clipboard_requests
        .lock()
        .map(|mut requests| {
            if requests.get(&key) == Some(&request) {
                false
            } else {
                requests.insert(key, request);
                true
            }
        })
        .unwrap_or(false);
    if changed {
        perform_clipboard_action(
            &context.runtime,
            context.window_id,
            clipboard,
            gpui::Modifiers::default(),
            context.cx,
        );
    }
}

#[cfg(feature = "real-gpui")]
fn perform_clipboard_action(
    runtime: &SharedRuntime,
    window_id: u64,
    clipboard: &(Option<String>, Option<String>, bool),
    modifiers: gpui::Modifiers,
    cx: &mut gpui::App,
) {
    if let (Some(event), Some(text), without_primary) = clipboard {
        let primary = if cfg!(target_os = "macos") {
            modifiers.platform
        } else {
            modifiers.control
        };
        if !without_primary || !primary {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.clone()));
            let _ = push_event(
                runtime,
                NativeEvent::PrimitiveClipboardWrite {
                    window_id,
                    event: event.clone(),
                },
            );
        }
    }
}

#[cfg(feature = "real-gpui")]
pub(crate) type PointerObservers = std::rc::Rc<
    std::cell::RefCell<
        std::collections::HashMap<String, (SharedRuntime, u64, String, gpui::Entity<bool>)>,
    >,
>;

#[cfg(feature = "real-gpui")]
pub(crate) fn dispatch_pointer_modifiers(
    observers: &PointerObservers,
    modifiers: gpui::Modifiers,
    cx: &gpui::App,
) {
    for (runtime, window_id, event, hovered) in observers.borrow().values() {
        if *hovered.read(cx) {
            emit_pointer_state(runtime, *window_id, event, true, modifiers);
        }
    }
}

#[cfg(feature = "real-gpui")]
#[derive(Default)]
pub(crate) struct BoundsCache {
    windows: std::collections::HashMap<u64, WindowBounds>,
}

#[cfg(feature = "real-gpui")]
#[derive(Default)]
struct WindowBounds {
    generation: u64,
    entries: std::collections::HashMap<String, (u64, crate::event::ElementBoundsGeometry)>,
}

#[cfg(feature = "real-gpui")]
impl BoundsCache {
    pub(crate) fn begin_frame(&mut self, window_id: u64) {
        let window = self.windows.entry(window_id).or_default();
        window.generation += 1;
        // Prepaint reports observers after root rendering. Keep the previous
        // frame's observers for deduplication, retiring IDs that disappeared.
        window
            .entries
            .retain(|_, (seen, _)| *seen + 1 >= window.generation);
    }

    fn observe(&mut self, window_id: u64, value: crate::event::ElementBoundsGeometry) -> bool {
        let window = self.windows.entry(window_id).or_default();
        let changed = window
            .entries
            .get(&value.id)
            .is_none_or(|(_, previous)| previous != &value);
        window
            .entries
            .insert(value.id.clone(), (window.generation, value));
        changed
    }

    pub(crate) fn forget_window(&mut self, window_id: u64) {
        self.windows.remove(&window_id);
    }
}

#[cfg(feature = "real-gpui")]
pub(crate) fn emit_bounds(
    runtime: &SharedRuntime,
    window_id: u64,
    id: &str,
    event: &str,
    bounds: gpui::Bounds<gpui::Pixels>,
) {
    let value = crate::event::ElementBoundsGeometry {
        id: id.to_string(),
        x: f32::from(bounds.origin.x) as f64,
        y: f32::from(bounds.origin.y) as f64,
        width: f32::from(bounds.size.width) as f64,
        height: f32::from(bounds.size.height) as f64,
        coordinate_space: "window_native_pixels".to_string(),
    };
    let changed = runtime
        .element_bounds
        .lock()
        .map(|mut known| known.observe(window_id, value.clone()))
        .unwrap_or(false);
    if changed {
        let _ = push_event(
            runtime,
            NativeEvent::Bounds {
                window_id,
                event: event.to_string(),
                value,
            },
        );
    }
}

#[cfg(all(test, feature = "real-gpui"))]
mod bounds_cache_tests {
    use super::BoundsCache;
    use crate::event::ElementBoundsGeometry;

    fn bounds(id: &str) -> ElementBoundsGeometry {
        ElementBoundsGeometry {
            id: id.to_string(),
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 20.0,
            coordinate_space: "window_native_pixels".to_string(),
        }
    }

    #[test]
    fn all_active_observers_report_and_deduplicate_beyond_256_ids() {
        let mut cache = BoundsCache::default();
        cache.begin_frame(1);
        for index in 0..1024 {
            assert!(cache.observe(1, bounds(&format!("row-{index}"))));
        }
        cache.begin_frame(1);
        for index in 0..1024 {
            assert!(!cache.observe(1, bounds(&format!("row-{index}"))));
        }
        assert!(cache.observe(1, bounds("new-sidebar")));
        let mut moved = bounds("row-0");
        moved.y = 30.0;
        assert!(cache.observe(1, moved));
    }

    #[test]
    fn retired_observers_are_pruned_without_repeating_live_bounds() {
        let mut cache = BoundsCache::default();
        for generation in 0..100 {
            cache.begin_frame(1);
            assert!(cache.observe(1, bounds(&format!("page-{generation}"))));
            assert_eq!(cache.observe(1, bounds("persistent")), generation == 0);
            assert!(cache.windows[&1].entries.len() <= 3);
        }
        assert!(cache.observe(1, bounds("page-0")));
    }

    #[test]
    fn windows_have_independent_frames_and_closing_retires_their_bounds() {
        let mut cache = BoundsCache::default();
        cache.begin_frame(1);
        assert!(cache.observe(1, bounds("same-id")));
        assert!(cache.observe(2, bounds("same-id")));
        for _ in 0..4 {
            cache.begin_frame(1);
        }
        assert!(!cache.observe(2, bounds("same-id")));
        cache.forget_window(2);
        assert!(!cache.windows.contains_key(&2));
        assert!(cache.observe(2, bounds("same-id")));
    }
}
