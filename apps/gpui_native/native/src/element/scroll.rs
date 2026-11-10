//! Persistent offsets and scrollbars for single-axis scroll containers.
use super::ElementRenderContext;
use crate::{gpui, push_event, EventValue, InputKind, NativeEvent};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub(crate) type Scrolls = Rc<RefCell<HashMap<String, Rc<RefCell<Scroll>>>>>;

#[derive(Default)]
pub(crate) struct Scroll {
    handle: gpui::ScrollHandle,
    horizontal: bool,
    request: u64,
    seen: bool,
    reported: Option<[f64; 3]>,
    event: Option<String>,
    grab: Option<f32>,
    hovered: bool,
}

pub(crate) fn begin_frame(scrolls: &Scrolls) {
    scrolls.borrow_mut().retain(|_, scroll| {
        let mut scroll = scroll.borrow_mut();
        let seen = scroll.seen;
        scroll.seen = false;
        seen
    });
}

pub(crate) struct ScrollOptions {
    pub(crate) position: f64,
    pub(crate) horizontal: bool,
    pub(crate) request: u64,
    pub(crate) event: Option<String>,
    pub(crate) bounds_event: Option<String>,
    pub(crate) scrollbar: bool,
    pub(crate) foreground: gpui::Hsla,
}

pub(crate) fn apply(
    element: gpui::Stateful<gpui::Div>,
    id: &str,
    options: ScrollOptions,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::Stateful<gpui::Div> {
    use gpui::{ParentElement, StatefulInteractiveElement, Styled};
    let ScrollOptions {
        position,
        horizontal,
        request,
        event,
        bounds_event,
        scrollbar,
        foreground,
    } = options;
    let shared = context
        .scrolls
        .borrow_mut()
        .entry(format!("{}-{id}", context.id_namespace))
        .or_default()
        .clone();
    let handle = {
        let mut scroll = shared.borrow_mut();
        scroll.seen = true;
        if scroll.horizontal != horizontal {
            scroll.horizontal = horizontal;
            scroll.request = 0;
            scroll
                .handle
                .set_offset(gpui::point(gpui::px(0.), gpui::px(0.)));
            scroll.reported = None;
        }
        if !scrollbar {
            scroll.grab = None;
            scroll.hovered = false;
        }
        if scroll.event != event {
            scroll.reported = None;
            scroll.event = event;
        }
        if request > 0 && scroll.request != request {
            scroll.request = request;
            scroll
                .handle
                .set_offset(point(horizontal, -position.max(0.0) as f32, 0.0));
        }
        scroll.handle.clone()
    };
    let runtime = context.runtime.clone();
    let window_id = context.window_id;
    let bounds_id = id.to_string();
    let reserved = scrollbar && main(horizontal, handle.max_offset()) > 0.0;
    let element = element.scrollbar_width(gpui::px(if reserved { 10.0 } else { 0.0 }));
    let paint_state = shared.clone();
    element.track_scroll(&handle).child(
        gpui::canvas(
            move |_, window, _| {
                let bounds = handle.bounds();
                if let Some(event) = &bounds_event {
                    super::event::emit_bounds(&runtime, window_id, &bounds_id, event, bounds);
                }
                let mut scroll = shared.borrow_mut();
                let values = [
                    f64::from(-main(horizontal, scroll.handle.offset())),
                    f64::from(length(horizontal, bounds.size)),
                    f64::from(main(horizontal, scroll.handle.max_offset())),
                ];
                if scrollbar && (values[2] > 0.0) != reserved {
                    window.refresh();
                }
                if scroll.reported != Some(values) {
                    scroll.reported = Some(values);
                    if let Some(event) = &scroll.event {
                        let _ = push_event(
                            &runtime,
                            NativeEvent::Input {
                                kind: InputKind::Change,
                                window_id,
                                event: event.clone(),
                                value: Some(EventValue::Numbers(values.to_vec())),
                            },
                        );
                    }
                }
                let height = length(horizontal, bounds.size);
                let maximum = values[2] as f32;
                if !scrollbar || maximum <= 0.0 || height <= 0.0 {
                    return None;
                }
                let length = (height * height / (height + maximum)).max(18.0).min(height);
                let start = (values[0] as f32).clamp(0.0, maximum) / maximum * (height - length);
                let track = if horizontal {
                    gpui::Bounds::new(
                        gpui::point(bounds.left(), bounds.bottom() - gpui::px(10.)),
                        gpui::size(bounds.size.width, gpui::px(10.)),
                    )
                } else {
                    gpui::Bounds::new(
                        gpui::point(bounds.right() - gpui::px(10.), bounds.top()),
                        gpui::size(gpui::px(10.), bounds.size.height),
                    )
                };
                let hitbox = window.insert_hitbox(track, gpui::HitboxBehavior::Normal);
                Some(BarGeometry {
                    horizontal,
                    track,
                    hitbox,
                    maximum,
                    length,
                    start,
                })
            },
            move |_, geometry, window, _| {
                let Some(geometry) = geometry else {
                    return;
                };
                paint_bar(paint_state.clone(), foreground, geometry, window);
            },
        )
        .absolute()
        .inset_0(),
    )
}

struct BarGeometry {
    horizontal: bool,
    track: gpui::Bounds<gpui::Pixels>,
    hitbox: gpui::Hitbox,
    maximum: f32,
    length: f32,
    start: f32,
}

fn main(horizontal: bool, point: gpui::Point<gpui::Pixels>) -> f32 {
    f32::from(if horizontal { point.x } else { point.y })
}
fn length(horizontal: bool, size: gpui::Size<gpui::Pixels>) -> f32 {
    f32::from(if horizontal { size.width } else { size.height })
}
fn point(horizontal: bool, position: f32, cross: f32) -> gpui::Point<gpui::Pixels> {
    if horizontal {
        gpui::point(gpui::px(position), gpui::px(cross))
    } else {
        gpui::point(gpui::px(cross), gpui::px(position))
    }
}
fn size(horizontal: bool, extent: f32, cross: f32) -> gpui::Size<gpui::Pixels> {
    if horizontal {
        gpui::size(gpui::px(extent), gpui::px(cross))
    } else {
        gpui::size(gpui::px(cross), gpui::px(extent))
    }
}
fn set_position(scroll: &Scroll, position: f32) -> bool {
    let maximum = main(scroll.horizontal, scroll.handle.max_offset()).max(0.0);
    let position = position.clamp(0.0, maximum);
    if (-main(scroll.horizontal, scroll.handle.offset()) - position).abs() < 0.01 {
        return false;
    }
    scroll
        .handle
        .set_offset(point(scroll.horizontal, -position, 0.));
    true
}

fn paint_bar(
    state: Rc<RefCell<Scroll>>,
    foreground: gpui::Hsla,
    geometry: BarGeometry,
    window: &mut gpui::Window,
) {
    let BarGeometry {
        horizontal,
        track,
        hitbox,
        maximum,
        length,
        start,
    } = geometry;
    let mut color = foreground;
    color.a *= if state.borrow().hovered || state.borrow().grab.is_some() {
        0.35
    } else {
        0.20
    };
    let thumb = gpui::Bounds::new(
        track.origin + point(horizontal, start + 2., 2.),
        size(horizontal, (length - 4.).max(0.), 6.),
    );
    window.paint_quad(gpui::fill(thumb, color).corner_radii(gpui::px(3.)));
    let thumb_hit = gpui::Bounds::new(
        track.origin + point(horizontal, start, 0.),
        size(horizontal, length, 10.),
    );

    let wheel_state = state.clone();
    let wheel_hitbox = hitbox.clone();
    window.on_mouse_event(move |event: &gpui::ScrollWheelEvent, phase, window, cx| {
        if phase.bubble() && wheel_hitbox.is_hovered(window) && track.contains(&event.position) {
            let delta = event.delta.pixel_delta(window.line_height());
            let scroll = wheel_state.borrow();
            if set_position(&scroll, -main(horizontal, scroll.handle.offset() + delta)) {
                window.refresh();
                cx.stop_propagation();
            }
        }
    });

    let move_hitbox = hitbox.clone();
    let down_state = state.clone();
    window.on_mouse_event(move |event: &gpui::MouseDownEvent, phase, window, cx| {
        if !phase.bubble() || event.button != gpui::MouseButton::Left || !hitbox.is_hovered(window)
        {
            return;
        }
        let y = main(horizontal, event.position - track.origin);
        let mut scroll = down_state.borrow_mut();
        if y >= start && y <= start + length {
            scroll.grab = Some(y - start);
            scroll.hovered = true;
        } else {
            let current = -main(horizontal, scroll.handle.offset());
            let height = if horizontal {
                f32::from(track.size.width)
            } else {
                f32::from(track.size.height)
            };
            let page = (height * 0.8).max(height - 40.0);
            set_position(&scroll, current + if y < start { -page } else { page });
        }
        window.prevent_default();
        window.refresh();
        cx.stop_propagation();
    });

    let move_state = state.clone();
    window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, window, cx| {
        if !phase.capture() {
            return;
        }
        let mut scroll = move_state.borrow_mut();
        let previous = scroll.hovered;
        scroll.hovered = move_hitbox.is_hovered(window) && thumb_hit.contains(&event.position);
        if let Some(grab) = scroll.grab {
            let travel = (if horizontal {
                f32::from(track.size.width)
            } else {
                f32::from(track.size.height)
            }) - length;
            let y = main(horizontal, event.position - track.origin) - grab;
            let top = if travel > 0.0 {
                (y / travel).clamp(0.0, 1.0) * maximum
            } else {
                0.0
            };
            set_position(&scroll, top);
            window.refresh();
            cx.stop_propagation();
        } else if previous != scroll.hovered {
            window.refresh();
        }
    });

    window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, window, cx| {
        if phase.capture()
            && event.button == gpui::MouseButton::Left
            && state.borrow_mut().grab.take().is_some()
        {
            window.refresh();
            cx.stop_propagation();
        }
    });
}
