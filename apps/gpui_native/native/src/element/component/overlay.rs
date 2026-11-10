pub(crate) mod dialog;
pub(crate) mod dropdown;
pub(crate) mod tooltip;
#[cfg(feature = "components")]
mod virtual_anchor;

use crate::{
    gpui, ElementRenderContext, PopoverComponentNode, PopoverContentComponentNode,
    PopoverTriggerComponentNode,
};

#[cfg(feature = "components")]
#[derive(Debug, PartialEq)]
pub(super) struct OverlayTriggerAccessibility {
    pub(super) label: String,
    pub(super) expanded: bool,
}

#[cfg(feature = "components")]
pub(super) fn overlay_trigger_accessibility(
    label: String,
    expanded: bool,
) -> OverlayTriggerAccessibility {
    OverlayTriggerAccessibility { label, expanded }
}

#[cfg(feature = "components")]
pub(crate) fn render(
    node: PopoverComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    use super::super::component_registry::{ComponentOverlayState, ComponentPopover};
    use super::super::controlled::ControlledBinding;
    use gpui::{
        InteractiveElement, IntoElement, MouseButton, ParentElement, Role,
        StatefulInteractiveElement, Styled,
    };
    use gpui_component::popover::Popover;
    use std::sync::{Arc, Mutex};

    let virtual_node = node.anchor_x.map(|_| node.clone());
    let parent_view = context.window.current_view();
    let (trigger, content) = match popover_slots(node.children) {
        Some(slots) => slots,
        None => return invalid_slots(),
    };

    if context.components.popover_mut(&node.id).is_none() {
        context.components.insert_popover(
            &node.id,
            ComponentPopover {
                restore_trigger_after_escape: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                overlay: ComponentOverlayState {
                    binding: Arc::new(Mutex::new(ControlledBinding::new(
                        node.change.clone(),
                        node.open,
                    ))),
                    effective_open: Arc::new(Mutex::new(node.open)),
                    trigger_focus: context.cx.focus_handle(),
                    content_focus: context.cx.focus_handle(),
                },
                previous_focus: context.window.focused(context.cx),
                rendered_open: node.open,
                hover: Arc::new(Mutex::new(Default::default())),
                escape: None,
            },
        );
        if node.open {
            // Initial content is painted after the trigger captures its anchor
            // bounds. Apply focus then, rather than focusing a not-yet-painted
            // subtree or skipping focus because it mounted already open.
            let initial = context
                .components
                .popover_mut(&node.id)
                .expect("new popover");
            let content_focus = initial.overlay.content_focus.clone();
            let effective_open = initial.overlay.effective_open.clone();
            let focus_on_open = node.focus_on_open;
            context.window.on_next_frame(move |window, cx| {
                if focus_on_open && effective_open.lock().is_ok_and(|open| *open) {
                    content_focus.focus(window, cx);
                }
                cx.notify(parent_view);
            });
        }
    }

    let popover = context
        .components
        .popover_mut(&node.id)
        .expect("component popover should exist");
    let force_open = popover
        .overlay
        .binding
        .lock()
        .map(|mut binding| {
            binding.event = node.change.clone();
            binding.reconcile(&node.open)
        })
        .unwrap_or(true);
    if force_open {
        if let Ok(mut effective_open) = popover.overlay.effective_open.lock() {
            *effective_open = node.open;
        }
    }
    let open = popover
        .overlay
        .effective_open
        .lock()
        .map(|open| *open)
        .unwrap_or(node.open);

    let newly_opened = open && !popover.rendered_open;
    if !open
        && popover.rendered_open
        && popover
            .restore_trigger_after_escape
            .swap(false, std::sync::atomic::Ordering::Relaxed)
    {
        let focus = popover.overlay.trigger_focus.clone();
        let effective_open = popover.overlay.effective_open.clone();
        // The toolkit's close transition restores its own previous focus during
        // this render. Restore the interactive trigger after that transition.
        context.window.on_next_frame(move |window, cx| {
            if effective_open.lock().is_ok_and(|open| !*open) {
                focus.focus(window, cx);
            }
        });
    }
    if popover.rendered_open != open {
        if open {
            if !popover
                .overlay
                .content_focus
                .contains_focused(context.window, context.cx)
            {
                popover.previous_focus = context.window.focused(context.cx);
                if node.focus_on_open {
                    popover
                        .overlay
                        .content_focus
                        .focus(context.window, context.cx);
                }
            }
        } else if popover
            .overlay
            .content_focus
            .contains_focused(context.window, context.cx)
        {
            popover
                .previous_focus
                .take()
                .unwrap_or_else(|| popover.overlay.trigger_focus.clone())
                .focus(context.window, context.cx);
        }
        popover.rendered_open = open;
    }

    let escape_key = (context.window_id, node.id.clone());
    if let Ok(mut popovers) = context.runtime.nonmodal_popovers.lock() {
        popovers.retain(|_, state| state.open.strong_count() > 0);
        if !node.focus_on_open || node.anchor_x.is_some() {
            let order = if newly_opened {
                context
                    .runtime
                    .next_popover_order
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            } else {
                popovers
                    .get(&escape_key)
                    .map(|state| state.order)
                    .unwrap_or(0)
            };
            popovers.insert(
                escape_key,
                crate::runtime::NonmodalPopover {
                    open: Arc::downgrade(&popover.overlay.effective_open),
                    order,
                    parent: node.parent_popover.clone(),
                    hover: Arc::downgrade(&popover.hover),
                },
            );
        } else {
            popovers.remove(&escape_key);
        }
    }
    if !node.focus_on_open || node.anchor_x.is_some() {
        if popover.escape.is_none() {
            let target_window = context.window.window_handle().window_id();
            let runtime = context.runtime.clone();
            let window_id = context.window_id;
            let binding = popover.overlay.binding.clone();
            let effective_open = popover.overlay.effective_open.clone();
            let hover = popover.hover.clone();
            let popover_id = node.id.clone();
            let restore = popover.restore_trigger_after_escape.clone();
            let restore_trigger = !node.trigger_passthrough && node.anchor_x.is_none();
            popover.escape = Some(context.cx.intercept_keystrokes(move |event, window, cx| {
                if window.window_handle().window_id() == target_window
                    && event.keystroke.key == "escape"
                    && runtime.popover_owns_escape(window_id, &popover_id)
                    && effective_open.lock().map(|open| *open).unwrap_or(false)
                {
                    if let Ok(mut state) = hover.lock() {
                        state.generation = state.generation.wrapping_add(1);
                    }
                    // GPUI invokes every keystroke interceptor before checking
                    // propagation. Keep Escape owned through this dispatch so
                    // a focused input cannot also handle the same dismissal.
                    if let Ok(mut windows) = runtime.consumed_overlay_escape.lock() {
                        windows.insert(window_id);
                    }
                    let consumed = runtime.clone();
                    cx.defer(move |_| {
                        if let Ok(mut windows) = consumed.consumed_overlay_escape.lock() {
                            windows.remove(&window_id);
                        }
                    });
                    if restore_trigger {
                        restore.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    request_open(&runtime, window_id, &binding, &effective_open, false, false);
                    window.refresh();
                    cx.stop_propagation();
                }
            }));
        }
    } else {
        popover.escape = None;
    }

    let hover = popover.hover.clone();
    let dismiss_hover = popover.hover.clone();
    let binding = popover.overlay.binding.clone();
    let effective_open = popover.overlay.effective_open.clone();
    let trigger_focus = popover.overlay.trigger_focus.clone();
    let content_focus = popover.overlay.content_focus.clone();
    let runtime = context.runtime.clone();
    let window_id = context.window_id;
    let trigger_runtime = runtime.clone();
    let trigger_binding = binding.clone();
    let trigger_effective_open = effective_open.clone();
    let mouse_focus = trigger_focus.clone();
    let accessibility = overlay_trigger_accessibility(node.label, open);
    // Hover cards decorate an existing link/control. Their wrapper must not
    // add another Tab stop, steal its pointer focus, or consume activation keys.
    let passive_hover = node.trigger_passthrough;
    let previous_owner = context.tooltip_focus_owner.clone();
    if !passive_hover {
        context.tooltip_focus_owner = Some(trigger_focus.clone());
    }
    let trigger_child = render_trigger(trigger, context);
    context.tooltip_focus_owner = previous_owner;
    let mut trigger_element = gpui::div()
        .id(format!("{}-trigger", node.id))
        .child(trigger_child);
    if passive_hover {
        // gpui-component's Popover toggles on its parent mouse-down listener.
        // Leave activation to the child link instead of opening the hover card.
        trigger_element =
            trigger_element.on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            });
    } else {
        trigger_element = trigger_element
            .role(Role::Button)
            .aria_label(accessibility.label)
            .aria_expanded(accessibility.expanded)
            .track_focus(&trigger_focus.tab_stop(true))
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                mouse_focus.focus(window, cx);
            })
            .on_key_down(move |event, _window, cx| {
                if !matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    return;
                }
                request_open(
                    &trigger_runtime,
                    window_id,
                    &trigger_binding,
                    &trigger_effective_open,
                    !open,
                    false,
                );
                cx.stop_propagation();
            });
    }

    if node.open_on_hover {
        let hover = hover.clone();
        let runtime = runtime.clone();
        let binding = binding.clone();
        let effective_open = effective_open.clone();
        let enter_delay = node.hover_delay;
        let close_delay = node.close_delay;
        let bounds_hover = hover.clone();
        trigger_element = trigger_element
            .relative()
            .child(
                gpui::canvas(
                    |_bounds, _window, _cx| (),
                    move |bounds, (), _window, _cx| {
                        if let Ok(mut state) = bounds_hover.lock() {
                            state.trigger_bounds = Some(bounds);
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .on_hover(move |entered, _window, cx| {
                request_hover(
                    &runtime,
                    window_id,
                    Some(parent_view),
                    &binding,
                    &effective_open,
                    &hover,
                    true,
                    *entered,
                    enter_delay,
                    close_delay,
                    cx,
                );
            });
    }

    let mut content_element = gpui::div()
        .relative()
        .id(format!("{}-content", node.id))
        .track_focus(&content_focus)
        .child(render_content(content, context));
    if node.open_on_hover {
        let runtime = runtime.clone();
        let binding = binding.clone();
        let effective_open = effective_open.clone();
        let close_delay = node.close_delay;
        let hover = hover.clone();
        content_element = content_element.child(
            gpui::canvas(
                |_bounds, _window, _cx| (),
                move |bounds, (), window, _cx| {
                    if let Ok(mut state) = hover.lock() {
                        state.content_bounds = Some(bounds);
                    }
                    let runtime = runtime.clone();
                    let binding = binding.clone();
                    let effective_open = effective_open.clone();
                    let hover = hover.clone();
                    let exit_runtime = runtime.clone();
                    let exit_binding = binding.clone();
                    let exit_effective = effective_open.clone();
                    let exit_hover = hover.clone();
                    window.on_mouse_event(
                        move |event: &gpui::MouseMoveEvent, phase, _window, cx| {
                            if phase == gpui::DispatchPhase::Capture {
                                let inside = hover
                                    .lock()
                                    .ok()
                                    .map(|state| {
                                        state
                                            .content_bounds
                                            .is_some_and(|bounds| bounds.contains(&event.position))
                                            || (effective_open.lock().is_ok_and(|open| *open)
                                                && hover_bridge_contains(
                                                    state.trigger_bounds,
                                                    state.content_bounds,
                                                    event.position,
                                                ))
                                    })
                                    .unwrap_or(false);
                                request_hover(
                                    &runtime,
                                    window_id,
                                    Some(parent_view),
                                    &binding,
                                    &effective_open,
                                    &hover,
                                    false,
                                    inside,
                                    0.0,
                                    close_delay,
                                    cx,
                                );
                            }
                        },
                    );
                    window.on_mouse_event(
                        move |_event: &gpui::MouseExitEvent, phase, _window, cx| {
                            if phase == gpui::DispatchPhase::Capture {
                                request_hover(
                                    &exit_runtime,
                                    window_id,
                                    Some(parent_view),
                                    &exit_binding,
                                    &exit_effective,
                                    &exit_hover,
                                    true,
                                    false,
                                    0.0,
                                    close_delay,
                                    cx,
                                );
                                request_hover(
                                    &exit_runtime,
                                    window_id,
                                    Some(parent_view),
                                    &exit_binding,
                                    &exit_effective,
                                    &exit_hover,
                                    false,
                                    false,
                                    0.0,
                                    close_delay,
                                    cx,
                                );
                            }
                        },
                    );
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        );
    }
    if let Some(virtual_node) = virtual_node {
        return virtual_anchor::build(
            &virtual_node,
            context,
            content_element,
            open,
            &binding,
            &effective_open,
            &hover,
        )
        .unwrap_or_else(invalid_slots);
    }
    let callback_runtime = runtime;
    let callback_binding = binding;
    let callback_effective_open = effective_open;
    let mut element = Popover::new(node.id)
        .anchor(anchor(node.anchor.as_deref()))
        .appearance(node.appearance)
        .overlay_closable(node.closable)
        .track_focus(&content_focus)
        .trigger(OverlayTrigger::new(trigger_element.into_any_element()))
        .on_open_change(move |open, _window, _cx| {
            if !*open {
                if let Ok(mut state) = dismiss_hover.lock() {
                    state.generation = state.generation.wrapping_add(1);
                }
            }
            request_open(
                &callback_runtime,
                window_id,
                &callback_binding,
                &callback_effective_open,
                *open,
                true,
            );
        })
        .child(content_element);
    if force_open || node.open_on_hover {
        element = element.open(open);
    }

    super::apply_component_styles(element, node.style).into_any_element()
}

// The narrow rectangular trough between a trigger and its popup remains
// interactive in both directions, even with a zero close delay. Match the
// reference/floating geometry rather than extending a dismissal timer.
#[cfg(feature = "components")]
fn hover_bridge_contains(
    trigger: Option<gpui::Bounds<gpui::Pixels>>,
    content: Option<gpui::Bounds<gpui::Pixels>>,
    position: gpui::Point<gpui::Pixels>,
) -> bool {
    let (Some(trigger), Some(content)) = (trigger, content) else {
        return false;
    };
    let buffer = gpui::px(1.0);
    let (left, right, top, bottom) = if content.top() >= trigger.bottom() {
        let narrow = if content.size.width > trigger.size.width {
            trigger
        } else {
            content
        };
        (
            narrow.left(),
            narrow.right(),
            trigger.bottom() - buffer,
            content.top() + buffer,
        )
    } else if content.bottom() <= trigger.top() {
        let narrow = if content.size.width > trigger.size.width {
            trigger
        } else {
            content
        };
        (
            narrow.left(),
            narrow.right(),
            content.bottom() - buffer,
            trigger.top() + buffer,
        )
    } else if content.left() >= trigger.right() {
        let narrow = if content.size.height > trigger.size.height {
            trigger
        } else {
            content
        };
        (
            trigger.right() - buffer,
            content.left() + buffer,
            narrow.top(),
            narrow.bottom(),
        )
    } else if content.right() <= trigger.left() {
        let narrow = if content.size.height > trigger.size.height {
            trigger
        } else {
            content
        };
        (
            content.right() - buffer,
            trigger.left() + buffer,
            narrow.top(),
            narrow.bottom(),
        )
    } else {
        return false;
    };
    position.x >= left && position.x <= right && position.y >= top && position.y <= bottom
}

#[cfg(feature = "components")]
#[allow(clippy::too_many_arguments)]
fn request_hover(
    runtime: &crate::SharedRuntime,
    window_id: u64,
    parent_view: Option<gpui::EntityId>,
    binding: &super::super::controlled::SharedBinding<bool>,
    effective_open: &std::sync::Arc<std::sync::Mutex<bool>>,
    hover: &std::sync::Arc<std::sync::Mutex<super::super::component_registry::PopoverHoverState>>,
    trigger: bool,
    entered: bool,
    enter_delay: f64,
    close_delay: f64,
    cx: &mut gpui::App,
) {
    let Ok(mut state) = hover.lock() else {
        return;
    };
    if (if trigger {
        state.trigger
    } else {
        state.content
    }) == entered
    {
        return;
    }
    if trigger {
        state.trigger = entered;
    } else {
        state.content = entered;
    }
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let desired = state.trigger || state.content;
    drop(state);
    let delay = if desired { enter_delay } else { close_delay };
    let runtime = runtime.clone();
    let binding = binding.clone();
    let effective_open = effective_open.clone();
    let weak_hover = std::sync::Arc::downgrade(hover);
    let timer = cx
        .background_executor()
        .timer(std::time::Duration::from_millis(
            delay.clamp(1.0, 60_000.0) as u64
        ));
    cx.spawn(async move |cx| {
        timer.await;
        cx.update(|cx| {
            let Some(hover) = weak_hover.upgrade() else {
                return;
            };
            let Ok(state) = hover.lock() else {
                return;
            };
            if state.generation != generation {
                return;
            }
            drop(state);
            if effective_open
                .lock()
                .map(|current| *current == desired)
                .unwrap_or(true)
            {
                return;
            }
            request_open(
                &runtime,
                window_id,
                &binding,
                &effective_open,
                desired,
                true,
            );
            if let Some(parent_view) = parent_view {
                cx.notify(parent_view);
            }
        });
    })
    .detach();
}

#[cfg(feature = "components")]
fn request_open(
    runtime: &crate::SharedRuntime,
    window_id: u64,
    binding: &super::super::controlled::SharedBinding<bool>,
    effective_open: &std::sync::Arc<std::sync::Mutex<bool>>,
    open: bool,
    native_applied: bool,
) {
    use crate::{push_event, EventValue, InputKind, NativeEvent};

    if let Ok(mut value) = effective_open.lock() {
        *value = open;
    }
    let event = binding.lock().ok().and_then(|mut binding| {
        binding.event.clone().inspect(|_event| {
            if native_applied {
                binding.push_pending(open);
            }
        })
    });
    let Some(event) = event else {
        return;
    };
    let result = push_event(
        runtime,
        NativeEvent::Input {
            kind: InputKind::Change,
            window_id,
            event,
            value: Some(EventValue::Boolean(open)),
        },
    );
    if result.is_err() && native_applied {
        if let Ok(mut binding) = binding.lock() {
            binding.pop_pending();
        }
    }
}

#[cfg(feature = "components")]
fn anchor(anchor: Option<&str>) -> gpui::Anchor {
    match anchor {
        Some("top_center") => gpui::Anchor::TopCenter,
        Some("top_right") => gpui::Anchor::TopRight,
        Some("bottom_left") => gpui::Anchor::BottomLeft,
        Some("bottom_center") => gpui::Anchor::BottomCenter,
        Some("bottom_right") => gpui::Anchor::BottomRight,
        Some("left_center") => gpui::Anchor::LeftCenter,
        Some("right_center") => gpui::Anchor::RightCenter,
        _other => gpui::Anchor::TopLeft,
    }
}

#[cfg(feature = "components")]
struct OverlayTrigger {
    element: gpui::AnyElement,
    selected: bool,
}

#[cfg(feature = "components")]
impl OverlayTrigger {
    fn new(element: gpui::AnyElement) -> Self {
        Self {
            element,
            selected: false,
        }
    }
}

#[cfg(feature = "components")]
impl gpui_component::Selectable for OverlayTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

#[cfg(feature = "components")]
impl gpui::IntoElement for OverlayTrigger {
    type Element = gpui::AnyElement;

    fn into_element(self) -> Self::Element {
        self.element
    }
}

fn popover_slots(
    children: Vec<crate::ElementNode>,
) -> Option<(PopoverTriggerComponentNode, PopoverContentComponentNode)> {
    let mut trigger = None;
    let mut content = None;

    for child in children {
        match child {
            crate::ElementNode::PopoverTriggerComponent(node) if trigger.is_none() => {
                trigger = Some(node);
            }
            crate::ElementNode::PopoverContentComponent(node) if content.is_none() => {
                content = Some(node);
            }
            _other => return None,
        }
    }

    Some((trigger?, content?))
}

pub(crate) fn render_trigger(
    node: PopoverTriggerComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    render_slot(node.style, node.children, context)
}

pub(crate) fn render_content(
    node: PopoverContentComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    render_slot(node.style, node.children, context)
}

pub(crate) fn render_dropdown_menu_trigger(
    node: crate::DropdownMenuTriggerComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    render_slot(node.style, node.children, context)
}

pub(crate) fn render_dropdown_menu_item(
    node: crate::DropdownMenuItemComponentNode,
    _context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    use gpui::{IntoElement, ParentElement};

    crate::apply_generated_render_styles(gpui::div(), node.style)
        .child(node.label)
        .into_any_element()
}

fn render_slot(
    style: crate::StyleAttrs,
    children: Vec<crate::ElementNode>,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    use gpui::{IntoElement, ParentElement};

    let mut element = crate::apply_generated_render_styles(gpui::div(), style);
    for child in children {
        element = element.child(child.render(context));
    }
    element.into_any_element()
}

fn invalid_slots() -> gpui::AnyElement {
    use gpui::{IntoElement, ParentElement};

    gpui::div()
        .child("invalid popover slots")
        .into_any_element()
}

#[cfg(all(test, feature = "components"))]
mod tests {
    use super::{overlay_trigger_accessibility, OverlayTriggerAccessibility};
    use crate::gpui;

    struct HoverFixture {
        runtime: crate::SharedRuntime,
        binding: super::super::super::controlled::SharedBinding<bool>,
        effective: std::sync::Arc<std::sync::Mutex<bool>>,
        hover: std::sync::Arc<
            std::sync::Mutex<super::super::super::component_registry::PopoverHoverState>,
        >,
    }

    impl HoverFixture {
        fn new() -> Self {
            Self {
                runtime: std::sync::Arc::new(crate::runtime::RuntimeState::new()),
                binding: std::sync::Arc::new(std::sync::Mutex::new(
                    super::super::super::controlled::ControlledBinding::new(
                        Some("hover".to_string()),
                        false,
                    ),
                )),
                effective: std::sync::Arc::new(std::sync::Mutex::new(false)),
                hover: std::sync::Arc::new(std::sync::Mutex::new(Default::default())),
            }
        }
        fn enter(&self, trigger: bool, entered: bool, cx: &mut gpui::TestAppContext) {
            cx.update(|cx| {
                super::request_hover(
                    &self.runtime,
                    7,
                    None,
                    &self.binding,
                    &self.effective,
                    &self.hover,
                    trigger,
                    entered,
                    50.0,
                    200.0,
                    cx,
                )
            });
            cx.run_until_parked();
        }
        fn advance(&self, milliseconds: u64, cx: &mut gpui::TestAppContext) {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(milliseconds));
            cx.run_until_parked();
        }
        fn open(&self) -> bool {
            *self.effective.lock().unwrap()
        }
    }

    #[gpui::test]
    fn hover_waits_for_the_delay_and_leaving_early_cancels_open(cx: &mut gpui::TestAppContext) {
        let fixture = HoverFixture::new();
        fixture.enter(true, true, cx);
        fixture.advance(49, cx);
        assert!(!fixture.open());
        fixture.enter(true, false, cx);
        fixture.advance(250, cx);
        assert!(!fixture.open());
        fixture.enter(true, true, cx);
        fixture.advance(50, cx);
        assert!(fixture.open());
    }

    #[gpui::test]
    fn entering_content_cancels_trigger_close_and_leaving_both_closes(
        cx: &mut gpui::TestAppContext,
    ) {
        let fixture = HoverFixture::new();
        fixture.enter(true, true, cx);
        fixture.advance(50, cx);
        assert!(fixture.open());
        fixture.enter(true, false, cx);
        fixture.advance(100, cx);
        fixture.enter(false, true, cx);
        fixture.advance(200, cx);
        assert!(fixture.open());
        fixture.enter(false, false, cx);
        fixture.advance(199, cx);
        assert!(fixture.open());
        fixture.advance(1, cx);
        assert!(!fixture.open());
    }

    #[gpui::test]
    fn removing_hover_state_cancels_pending_activation(cx: &mut gpui::TestAppContext) {
        let fixture = HoverFixture::new();
        fixture.enter(true, true, cx);
        let effective = fixture.effective.clone();
        drop(fixture);
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(100));
        cx.run_until_parked();
        assert!(!*effective.lock().unwrap());
    }

    #[gpui::test]
    fn rendered_trigger_hover_requests_open(cx: &mut gpui::TestAppContext) {
        use gpui::{point, px, size, IntoElement, Modifiers};
        cx.update(gpui_component::init);
        let runtime = std::sync::Arc::new(crate::runtime::RuntimeState::new());
        let mut trigger_style = crate::StyleAttrs::default();
        trigger_style.width = Some(px(100.0).into());
        trigger_style.height = Some(px(30.0).into());
        let mut content_style = crate::StyleAttrs::default();
        content_style.width = Some(px(100.0).into());
        content_style.height = Some(px(80.0).into());
        let root = crate::ElementNode::PopoverComponent(crate::PopoverComponentNode {
            style: crate::StyleAttrs::default(),
            id: "preview".into(),
            label: "Preview".into(),
            open: false,
            open_on_hover: true,
            focus_on_open: false,
            trigger_passthrough: false,
            hover_delay: 0.0,
            close_delay: 0.0,
            anchor_x: None,
            anchor_y: None,
            anchor_width: None,
            anchor_height: None,
            anchor_hovered: false,
            parent_popover: None,
            anchor: Some("top_left".into()),
            appearance: true,
            closable: true,
            change: Some("preview".into()),
            children: vec![
                crate::ElementNode::PopoverTriggerComponent(crate::PopoverTriggerComponentNode {
                    style: trigger_style,
                    children: vec![],
                }),
                crate::ElementNode::PopoverContentComponent(crate::PopoverContentComponentNode {
                    style: content_style,
                    children: vec![],
                }),
            ],
        });
        let state = std::sync::Arc::new(crate::WindowState::new(root, Vec::new()));
        let view_runtime = runtime.clone();
        let (view, cx) = cx.add_window_view(move |_window, _cx| {
            crate::ElixirRoot::new(state, view_runtime, 7, false, false)
        });
        cx.draw(
            point(px(0.0), px(0.0)),
            size(px(400.0), px(300.0)),
            |_window, _cx| view.clone().into_any_element(),
        );
        cx.simulate_mouse_move(point(px(300.0), px(200.0)), None, Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_mouse_move(point(px(5.0), px(5.0)), None, Modifiers::default());
        cx.run_until_parked();
        cx.cx
            .executor()
            .advance_clock(std::time::Duration::from_millis(1));
        cx.run_until_parked();
        let events = runtime.events.lock().unwrap();
        assert!(events.iter().any(|event| matches!(event, crate::NativeEvent::Input {event, value: Some(crate::EventValue::Boolean(true)), ..} if event == "preview")), "{events:?}");
        drop(events);
        view.update(cx, |_view, cx| cx.notify());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let content_bounds = view
            .update(cx, |view, _cx| {
                view.components
                    .popover_mut("preview")
                    .unwrap()
                    .hover
                    .lock()
                    .unwrap()
                    .content_bounds
            })
            .expect("popup content was painted");
        assert!(
            content_bounds.origin.y < px(100.0),
            "sensor must overlay the content rather than follow it: {content_bounds:?}"
        );
        let trigger_bounds = view.update(cx, |view, _cx| {
            view.components
                .popover_mut("preview")
                .unwrap()
                .hover
                .lock()
                .unwrap()
                .trigger_bounds
                .unwrap()
        });
        let gap = point(
            content_bounds.center().x,
            (trigger_bounds.bottom() + content_bounds.top()) / 2.0,
        );
        assert!(
            super::hover_bridge_contains(Some(trigger_bounds), Some(content_bounds), gap),
            "trigger {trigger_bounds:?}, content {content_bounds:?}, gap {gap:?}"
        );
        cx.simulate_mouse_move(gap, None, Modifiers::default());
        cx.run_until_parked();
        cx.cx
            .executor()
            .advance_clock(std::time::Duration::from_millis(50));
        cx.run_until_parked();
        assert!(view.update(cx, |view, _cx| {
            *view
                .components
                .popover_mut("preview")
                .unwrap()
                .overlay
                .effective_open
                .lock()
                .unwrap()
        }));
        cx.simulate_mouse_move(content_bounds.center(), None, Modifiers::default());
        cx.run_until_parked();
        let content_hovered = view.update(cx, |view, _cx| {
            view.components
                .popover_mut("preview")
                .unwrap()
                .hover
                .lock()
                .unwrap()
                .content
        });
        let bounds = view.update(cx, |view, _cx| {
            view.components
                .popover_mut("preview")
                .unwrap()
                .hover
                .lock()
                .unwrap()
                .content_bounds
        });
        assert!(
            content_hovered,
            "rendered content must receive pointer entry, bounds {bounds:?}"
        );
        cx.cx
            .executor()
            .advance_clock(std::time::Duration::from_millis(250));
        cx.run_until_parked();
        let still_open = view.update(cx, |view, _cx| {
            *view
                .components
                .popover_mut("preview")
                .unwrap()
                .overlay
                .effective_open
                .lock()
                .unwrap()
        });
        assert!(still_open);
        cx.simulate_mouse_move(point(px(300.0), px(200.0)), None, Modifiers::default());
        cx.cx
            .executor()
            .advance_clock(std::time::Duration::from_millis(1));
        cx.run_until_parked();
        assert!(!view.update(cx, |view, _cx| {
            *view
                .components
                .popover_mut("preview")
                .unwrap()
                .overlay
                .effective_open
                .lock()
                .unwrap()
        }));
    }

    #[test]
    fn hover_bridge_follows_the_narrower_edge_on_each_side() {
        use gpui::{point, px, size, Bounds};
        let trigger = Some(Bounds::new(
            point(px(100.0), px(100.0)),
            size(px(20.0), px(20.0)),
        ));
        for (x, y, width, height, inside, outside) in [
            (40.0, 124.0, 100.0, 60.0, (110.0, 122.0), (90.0, 122.0)),
            (40.0, 36.0, 100.0, 60.0, (110.0, 98.0), (90.0, 98.0)),
            (124.0, 40.0, 60.0, 100.0, (122.0, 110.0), (122.0, 90.0)),
            (36.0, 40.0, 60.0, 100.0, (98.0, 110.0), (98.0, 90.0)),
        ] {
            let content = Some(Bounds::new(
                point(px(x), px(y)),
                size(px(width), px(height)),
            ));
            assert!(super::hover_bridge_contains(
                trigger,
                content,
                point(px(inside.0), px(inside.1))
            ));
            assert!(!super::hover_bridge_contains(
                trigger,
                content,
                point(px(outside.0), px(outside.1))
            ));
        }
        assert!(!super::hover_bridge_contains(
            trigger,
            None,
            point(px(110.0), px(122.0))
        ));
    }

    #[test]
    fn trigger_accessibility_tracks_label_and_expanded_state() {
        assert_eq!(
            overlay_trigger_accessibility("Account".to_string(), true),
            OverlayTriggerAccessibility {
                label: "Account".to_string(),
                expanded: true,
            }
        );
        assert!(!overlay_trigger_accessibility("Account".to_string(), false).expanded);
    }
}

#[cfg(not(feature = "components"))]
pub(crate) fn render(
    node: PopoverComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    let (trigger, content) = match popover_slots(node.children) {
        Some(slots) => slots,
        None => return invalid_slots(),
    };

    let children = vec![
        render_trigger(trigger, context),
        render_content(content, context),
    ];
    use gpui::{IntoElement, ParentElement};
    let mut element = crate::apply_generated_render_styles(gpui::div(), node.style);
    for child in children {
        element = element.child(child);
    }
    element.into_any_element()
}
