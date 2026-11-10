//! Controlled image/diagram viewport with pointer-anchored zoom and captured pans.
use super::super::controlled::ControlledBinding;
use crate::{
    gpui, ElementRenderContext, EventValue, InputKind, NativeEvent, PanZoomComponentNode,
    SharedRuntime,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq)]
struct View {
    x: f64,
    y: f64,
    scale: f64,
}
impl View {
    fn zoom(&mut self, factor: f64, pointer: [f64; 2], min: f64, max: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let scale = (self.scale * factor).clamp(min, max);
        let ratio = scale / self.scale;
        self.x = pointer[0] - (pointer[0] - self.x) * ratio;
        self.y = pointer[1] - (pointer[1] - self.y) * ratio;
        self.scale = scale;
    }
}
const START: View = View {
    x: 0.0,
    y: 0.0,
    scale: 1.0,
};
struct Drag {
    point: gpui::Point<gpui::Pixels>,
    moved: bool,
}
struct State {
    view: View,
    binding: ControlledBinding<View>,
    reset: u64,
    bounds: gpui::Bounds<gpui::Pixels>,
    dimensions: [f64; 2],
    min: f64,
    max: f64,
    zoomable: bool,
    drag: Option<Drag>,
    pressed_outside: Option<bool>,
    pinch_start: Option<View>,
}
impl State {
    fn content_bounds(&self) -> gpui::Bounds<gpui::Pixels> {
        let width = f64::from(f32::from(self.bounds.size.width));
        let height = f64::from(f32::from(self.bounds.size.height));
        let fit = (width * 0.9 / self.dimensions[0])
            .min(height * 0.9 / self.dimensions[1])
            .min(1.0);
        let w = self.dimensions[0] * fit * self.view.scale;
        let h = self.dimensions[1] * fit * self.view.scale;
        gpui::Bounds::new(
            self.bounds.origin
                + gpui::point(
                    gpui::px(((width - w) / 2.0 + self.view.x) as f32),
                    gpui::px(((height - h) / 2.0 + self.view.y) as f32),
                ),
            gpui::size(gpui::px(w as f32), gpui::px(h as f32)),
        )
    }
    fn pointer(&self, point: gpui::Point<gpui::Pixels>) -> [f64; 2] {
        let delta = point - self.bounds.center();
        [f64::from(f32::from(delta.x)), f64::from(f32::from(delta.y))]
    }
    fn zoom(&mut self, factor: f64, point: gpui::Point<gpui::Pixels>) {
        self.view
            .zoom(factor, self.pointer(point), self.min, self.max);
    }
}
pub(crate) struct ComponentPanZoom {
    state: Rc<RefCell<State>>,
}
#[derive(Clone)]
struct Controller {
    state: Rc<RefCell<State>>,
    runtime: SharedRuntime,
    window_id: u64,
    root: gpui::EntityId,
    click: Option<String>,
}
impl Controller {
    fn changed(&self, before: View, cx: &mut gpui::App) {
        let mut state = self.state.borrow_mut();
        if state.view == before {
            return;
        }
        let view = state.view;
        state.binding.push_pending(view);
        if let Some(event) = state.binding.event.clone() {
            let _ = crate::push_event(
                &self.runtime,
                NativeEvent::Input {
                    kind: InputKind::Change,
                    window_id: self.window_id,
                    event,
                    value: Some(EventValue::Numbers(vec![view.x, view.y, view.scale])),
                },
            );
        }
        cx.notify(self.root);
    }
    fn wheel(&self, event: &gpui::ScrollWheelEvent, window: &mut gpui::Window, cx: &mut gpui::App) {
        let mut state = self.state.borrow_mut();
        if !state.zoomable {
            return;
        }
        let before = state.view;
        let delta = event.delta.pixel_delta(window.line_height());
        if event.modifiers.control || event.modifiers.platform {
            if state.pinch_start.is_none() {
                // Native wheel deltas have the opposite sign to DOM WheelEvent.
                state.zoom(
                    (f64::from(f32::from(delta.y)).clamp(-20.0, 20.0) / 100.0).exp(),
                    event.position,
                );
            }
        } else {
            state.view.x += f64::from(f32::from(delta.x));
            state.view.y += f64::from(f32::from(delta.y));
        }
        drop(state);
        self.changed(before, cx);
        cx.stop_propagation();
    }
    fn pinch(&self, event: &gpui::PinchEvent, cx: &mut gpui::App) {
        let mut state = self.state.borrow_mut();
        if !state.zoomable {
            return;
        }
        let before = state.view;
        match event.phase {
            gpui::TouchPhase::Cancelled => {
                if let Some(start) = state.pinch_start.take() {
                    state.view = start;
                }
            }
            gpui::TouchPhase::Ended => {
                state.pinch_start = None;
            }
            _ => {
                if state.pinch_start.is_none() {
                    state.pinch_start = Some(before);
                }
                state.zoom(1.0 + f64::from(event.delta), event.position);
            }
        }
        drop(state);
        self.changed(before, cx);
        cx.stop_propagation();
    }
    fn down(&self, event: &gpui::MouseDownEvent, cx: &mut gpui::App) {
        let mut state = self.state.borrow_mut();
        state.pressed_outside = Some(!state.content_bounds().contains(&event.position));
        let before = state.view;
        if state.zoomable {
            state.drag = Some(Drag {
                point: event.position,
                moved: false,
            });
            if event.click_count == 2 {
                if state.view == START {
                    state.zoom(2.0, event.position);
                } else {
                    state.view = START;
                }
            }
        }
        drop(state);
        self.changed(before, cx);
        cx.stop_propagation();
    }
    fn movement(&self, event: &gpui::MouseMoveEvent, cx: &mut gpui::App) {
        let mut state = self.state.borrow_mut();
        let Some(drag) = &state.drag else {
            return;
        };
        let delta = event.position - drag.point;
        let dx = f64::from(f32::from(delta.x));
        let dy = f64::from(f32::from(delta.y));
        if !drag.moved && dx.hypot(dy) < 4.0 {
            return;
        }
        let before = state.view;
        state.drag = Some(Drag {
            point: event.position,
            moved: true,
        });
        state.view.x += dx;
        state.view.y += dy;
        drop(state);
        self.changed(before, cx);
        cx.stop_propagation();
    }
    fn up(&self, cx: &mut gpui::App) {
        let mut state = self.state.borrow_mut();
        let moved = state.drag.take().is_some_and(|drag| drag.moved);
        let outside = state.pressed_outside.take();
        drop(state);
        if outside == Some(true) && !moved {
            if let Some(event) = &self.click {
                let _ = crate::push_event(
                    &self.runtime,
                    NativeEvent::Click {
                        window_id: self.window_id,
                        event: event.clone(),
                    },
                );
            }
        }
        if outside.is_some() {
            cx.stop_propagation();
        }
    }
}

pub(crate) fn render_pan_zoom(
    node: PanZoomComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    use gpui::{
        canvas, div, point, px, InteractiveElement, IntoElement, MouseButton, ParentElement,
        StatefulInteractiveElement, Styled,
    };
    let value = View {
        x: node.pan_x,
        y: node.pan_y,
        scale: node.scale,
    };
    if context.components.pan_zoom_mut(&node.id).is_none() {
        context.components.insert_pan_zoom(
            &node.id,
            ComponentPanZoom {
                state: Rc::new(RefCell::new(State {
                    view: value,
                    binding: ControlledBinding::new(node.change.clone(), value),
                    reset: node.reset_request,
                    bounds: gpui::Bounds::new(
                        point(px(0.0), px(0.0)),
                        context.window.viewport_size(),
                    ),
                    dimensions: [node.content_width, node.content_height],
                    min: node.min_scale,
                    max: node.max_scale,
                    zoomable: node.zoomable,
                    drag: None,
                    pressed_outside: None,
                    pinch_start: None,
                })),
            },
        );
    }
    let state = context
        .components
        .pan_zoom_mut(&node.id)
        .expect("pan/zoom state exists")
        .state
        .clone();
    {
        let mut state = state.borrow_mut();
        state.binding.event = node.change;
        let force = state.binding.reconcile(&value);
        if (force && value != state.view) || state.reset != node.reset_request {
            state.view = value;
            state.drag = None;
            state.pressed_outside = None;
            state.pinch_start = None;
            state.reset = node.reset_request;
            state.binding = ControlledBinding::new(state.binding.event.clone(), value);
        }
        state.dimensions = [node.content_width, node.content_height];
        state.min = node.min_scale;
        state.max = node.max_scale;
        state.zoomable = node.zoomable;
    }
    let control = Controller {
        state: state.clone(),
        runtime: context.runtime.clone(),
        window_id: context.window_id,
        root: context.cx.entity_id(),
        click: node.click,
    };
    let content = state.borrow().content_bounds();
    let origin = content.origin - state.borrow().bounds.origin;
    let observer = control.clone();
    let drag = control.clone();
    let observed = canvas(
        move |bounds, _, cx| {
            let mut state = observer.state.borrow_mut();
            let changed = state.bounds.size != bounds.size;
            state.bounds = bounds;
            if changed {
                cx.notify(observer.root);
            }
        },
        move |_, _, window, _| {
            let moving = drag.clone();
            window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, _, cx| {
                if phase.capture() {
                    moving.movement(event, cx);
                }
            });
            let up = drag.clone();
            window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, _, cx| {
                if phase.capture() && event.button == MouseButton::Left {
                    up.up(cx);
                }
            });
        },
    )
    .absolute()
    .size_full();
    let down = control.clone();
    let wheel = control.clone();
    let pinch = control.clone();
    let children = node
        .children
        .into_iter()
        .map(|child| child.render(context))
        .collect::<Vec<_>>();
    let selector = node.id.clone();
    super::apply_component_styles(div().relative().size_full().overflow_hidden(), node.style)
        .id(node.id)
        .debug_selector(|| selector)
        .role(gpui::Role::Group)
        .aria_label(node.label)
        .on_mouse_down(MouseButton::Left, move |event, _, cx| down.down(event, cx))
        .on_scroll_wheel(move |event, window, cx| wheel.wheel(event, window, cx))
        .on_pinch(move |event, _, cx| pinch.pinch(event, cx))
        .child(observed)
        .child(
            div()
                .absolute()
                .left(origin.x)
                .top(origin.y)
                .w(content.size.width)
                .h(content.size.height)
                .children(children),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zoom_preserves_the_content_point_beneath_the_pointer() {
        let mut view = View {
            x: 30.0,
            y: -10.0,
            scale: 1.0,
        };
        let before = [(100.0 - view.x) / view.scale, (50.0 - view.y) / view.scale];
        view.zoom(2.0, [100.0, 50.0], 0.25, 10.0);
        assert_eq!(
            view,
            View {
                x: -40.0,
                y: -70.0,
                scale: 2.0
            }
        );
        assert_eq!(
            before,
            [(100.0 - view.x) / view.scale, (50.0 - view.y) / view.scale]
        );
    }
    #[test]
    fn clamped_zoom_uses_the_actual_ratio_and_ignores_invalid_factors() {
        let mut view = START;
        view.zoom(100.0, [20.0, 30.0], 0.25, 10.0);
        assert_eq!(
            view,
            View {
                x: -180.0,
                y: -270.0,
                scale: 10.0
            }
        );
        view.zoom(0.0001, [20.0, 30.0], 0.25, 10.0);
        assert_eq!(view.scale, 0.25);
        let before = view;
        view.zoom(f64::NAN, [0.0, 0.0], 0.25, 10.0);
        assert_eq!(view, before);
    }
}
