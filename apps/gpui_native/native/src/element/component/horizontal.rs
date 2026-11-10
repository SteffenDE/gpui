//! Synchronized horizontal viewports, with independent pane extents.
use crate::{gpui, SharedRuntime};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

pub(crate) type Groups = Rc<RefCell<HashMap<String, Rc<RefCell<Group>>>>>;

struct Pane {
    handle: gpui::ScrollHandle,
    extent: f32,
    viewport: f32,
    event: Option<String>,
    active: bool,
    seen: bool,
    applied: Cell<f32>,
    reported: Option<[f64; 4]>,
    dragging: Rc<Cell<bool>>,
    drag_grab: Rc<Cell<Option<f32>>>,
    hovered: Rc<Cell<bool>>,
}
impl Default for Pane {
    fn default() -> Self {
        Self {
            handle: gpui::ScrollHandle::new(),
            extent: 0.0,
            viewport: 0.0,
            event: None,
            active: false,
            seen: false,
            applied: Cell::new(0.0),
            reported: None,
            dragging: Rc::new(Cell::new(false)),
            drag_grab: Rc::new(Cell::new(None)),
            hovered: Rc::new(Cell::new(false)),
        }
    }
}
impl Pane {
    fn max_left(&self) -> f32 {
        (self.extent - self.viewport).max(0.0)
    }
    fn left(&self) -> f32 {
        -f32::from(self.handle.offset().x)
    }
    fn set_left(&self, value: f32) {
        let left = value.clamp(0.0, self.max_left());
        self.applied.set(left);
        self.handle
            .set_offset(gpui::point(gpui::px(-left), gpui::px(0.0)));
    }
}

pub(crate) struct Group {
    revision: u64,
    position: f32,
    panes: HashMap<String, Pane>,
    runtime: SharedRuntime,
    window_id: u64,
}
impl Group {
    fn scroll_to(&mut self, source: &str, left: f32) -> bool {
        let Some(pane) = self.panes.get(source) else {
            return false;
        };
        let left = left.clamp(0.0, pane.max_left());
        let changed = (pane.left() - left).abs() > 0.01;
        if !changed {
            return false;
        }
        self.apply_left(left);
        true
    }
    fn apply_left(&mut self, left: f32) {
        self.position = left;
        for pane in self.panes.values() {
            pane.set_left(left);
        }
        self.report();
    }
    fn sync_from_handle(&mut self, source: &str) -> bool {
        let pane = &self.panes[source];
        let left = pane.left().clamp(0.0, pane.max_left());
        if (left - pane.applied.get()).abs() < 0.01 {
            return false;
        }
        self.apply_left(left);
        true
    }
    fn report(&mut self) {
        for pane in self.panes.values_mut() {
            let values = [
                pane.left() as f64,
                pane.max_left() as f64,
                pane.viewport as f64,
                pane.extent as f64,
            ];
            if !pane.active {
                continue;
            }
            let Some(event) = &pane.event else {
                continue;
            };
            if pane.reported == Some(values) {
                continue;
            }
            pane.reported = Some(values);
            let _ = crate::push_event(
                &self.runtime,
                crate::NativeEvent::Input {
                    kind: crate::InputKind::Change,
                    window_id: self.window_id,
                    event: event.clone(),
                    value: Some(crate::EventValue::Numbers(values.to_vec())),
                },
            );
        }
    }
}

pub(crate) fn begin_frame(groups: &Groups) {
    for group in groups.borrow().values() {
        for pane in group.borrow_mut().panes.values_mut() {
            pane.active = false;
            pane.seen = false;
        }
    }
}

#[derive(Clone)]
pub(crate) struct Binding {
    group: Rc<RefCell<Group>>,
    pane: String,
    view: gpui::EntityId,
}
pub(crate) struct ScrollbarOptions {
    pub(crate) enabled: bool,
    pub(crate) overlay: bool,
    pub(crate) outside: bool,
    pub(crate) content_padding: [f32; 4],
    pub(crate) visible: bool,
    pub(crate) gutter: f32,
    pub(crate) only: bool,
    pub(crate) color: Option<u32>,
}

impl Binding {
    pub(crate) fn configure(
        node: &crate::RichTextComponentNode,
        context: &crate::ElementRenderContext<'_, '_>,
    ) -> Option<Self> {
        let id = node.scroll_group.as_ref()?;
        let pane_id = if node.scroll_pane.is_empty() {
            "default".to_string()
        } else {
            node.scroll_pane.clone()
        };
        if id.is_empty() {
            return None;
        }
        let group = context
            .horizontal_groups
            .borrow_mut()
            .entry(id.clone())
            .or_insert_with(|| {
                Rc::new(RefCell::new(Group {
                    revision: node.scroll_revision,
                    position: 0.0,
                    panes: HashMap::new(),
                    runtime: context.runtime.clone(),
                    window_id: context.window_id,
                }))
            })
            .clone();
        {
            let mut group = group.borrow_mut();
            if group.revision != node.scroll_revision {
                group.revision = node.scroll_revision;
                for pane in group.panes.values_mut() {
                    pane.extent = 0.0;
                    pane.reported = None;
                }
            }
            let pane = group.panes.entry(pane_id.clone()).or_default();
            pane.seen = true;
            if let Some(event) = &node.scroll {
                pane.active = true;
                if pane.event.as_ref() != Some(event) {
                    pane.reported = None;
                }
                pane.event = Some(event.clone());
            }
        }
        Some(Self {
            group,
            pane: pane_id,
            view: context.cx.entity_id(),
        })
    }
    pub(crate) fn measured(&self, width: f32, window: &mut gpui::Window, cx: &mut gpui::App) {
        let changed = {
            let mut group = self.group.borrow_mut();
            let pane = group
                .panes
                .get_mut(&self.pane)
                .expect("registered horizontal pane");
            if width > pane.extent + 0.01 {
                pane.extent = width;
                true
            } else {
                false
            }
        };
        if changed {
            cx.notify(self.view);
            window.request_animation_frame();
        }
    }
    fn viewport(&self, width: f32) -> bool {
        let was_overflow = self.group.borrow().panes[&self.pane].max_left() > 0.0;
        let mut group = self.group.borrow_mut();
        let pane = group
            .panes
            .get_mut(&self.pane)
            .expect("registered horizontal pane");
        pane.viewport = width;
        if let Some(source) = group.panes.iter().find_map(|(id, pane)| {
            (pane.dragging.get() && (pane.left() - pane.applied.get()).abs() > 0.01)
                .then(|| id.clone())
        }) {
            group.sync_from_handle(&source);
        }
        if group
            .panes
            .values()
            .filter(|pane| pane.seen)
            .all(|pane| pane.viewport > 0.0)
        {
            let maximum = group
                .panes
                .values()
                .filter(|pane| pane.seen)
                .map(Pane::max_left)
                .fold(0.0_f32, f32::max);
            group.position = group.position.min(maximum);
        }
        let position = group.position;
        for pane in group.panes.values() {
            pane.set_left(position);
        }
        group.report();
        was_overflow != (group.panes[&self.pane].max_left() > 0.0)
    }
    fn wheel(&self, event: &gpui::ScrollWheelEvent, window: &mut gpui::Window, cx: &mut gpui::App) {
        let delta = event.delta.pixel_delta(window.line_height());
        let x = if event.modifiers.shift && f32::from(delta.x).abs() < 0.01 {
            delta.y
        } else if delta.x.abs() >= delta.y.abs() {
            delta.x
        } else {
            gpui::px(0.0)
        };
        if f32::from(x).abs() < 0.01 {
            return;
        }
        let mut group = self.group.borrow_mut();
        let left = group.panes[&self.pane].left() - f32::from(x);
        if group.scroll_to(&self.pane, left) {
            cx.notify(self.view);
            cx.stop_propagation();
        }
    }
    pub(crate) fn reveal_range(
        &self,
        start: f32,
        end: f32,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> bool {
        let group = self.group.borrow();
        let pane = &group.panes[&self.pane];
        let bounds = pane.handle.bounds();
        if pane.viewport <= 0.0 || f32::from(bounds.size.width) <= 0.0 {
            return false;
        }
        let mask = window.content_mask().bounds;
        let left = f32::from(bounds.left()).max(f32::from(mask.left()));
        let right = f32::from(bounds.right()).min(f32::from(mask.right()));
        if right <= left {
            return false;
        }
        if start >= left && end <= right {
            return true;
        }
        let target = pane.left() + (start + end - left - right) / 2.0;
        drop(group);
        let binding = self.clone();
        window.defer(cx, move |_window, cx| {
            if binding.group.borrow_mut().scroll_to(&binding.pane, target) {
                cx.notify(binding.view);
            }
        });
        true
    }

    pub(crate) fn key(
        &self,
        event: &gpui::KeyDownEvent,
        _window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> bool {
        let modifiers = event.keystroke.modifiers;
        if modifiers.platform || modifiers.control || modifiers.alt || modifiers.shift {
            return false;
        }
        let mut group = self.group.borrow_mut();
        let pane = &group.panes[&self.pane];
        let left = match event.keystroke.key.as_str() {
            "arrowleft" | "left" => pane.left() - 40.0,
            "arrowright" | "right" => pane.left() + 40.0,
            "home" => 0.0,
            "end" => pane.max_left(),
            _ => return false,
        };
        if group.scroll_to(&self.pane, left) {
            cx.notify(self.view);
        }
        true
    }
    pub(crate) fn viewport_element(
        &self,
        id: String,
        base: gpui::Stateful<gpui::Div>,
        content: gpui::AnyElement,
        scrollbar: ScrollbarOptions,
    ) -> gpui::AnyElement {
        use gpui::{
            InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
        };
        use gpui_component::ActiveTheme;
        let track_height = if scrollbar.overlay && !scrollbar.outside {
            6.0
        } else {
            10.0
        };
        let (handle, extent, dragging, grab, overflowing, hovered) = {
            let group = self.group.borrow();
            let pane = &group.panes[&self.pane];
            (
                pane.handle.clone(),
                pane.extent,
                pane.dragging.clone(),
                pane.drag_grab.clone(),
                pane.max_left() > 0.0,
                pane.hovered.clone(),
            )
        };
        let observer_binding = self.clone();
        let paint_binding = self.clone();
        let observer = gpui::canvas(
            move |bounds, window, cx| {
                if observer_binding
                    .viewport((f32::from(bounds.size.width) - scrollbar.gutter).max(0.0))
                    && !scrollbar.overlay
                {
                    let view = observer_binding.view;
                    window.defer(cx, move |_, cx| cx.notify(view));
                }
                if !scrollbar.enabled {
                    return None;
                }
                let group = observer_binding.group.borrow();
                let pane = &group.panes[&observer_binding.pane];
                let maximum = pane.max_left();
                if maximum <= 0.0 {
                    return None;
                }
                let width = f32::from(bounds.size.width);
                let length = (width * width / (pane.extent + scrollbar.gutter))
                    .max(17.0)
                    .min(width);
                let start = pane.left() / maximum * (width - length);
                let track = gpui::Bounds::new(
                    gpui::point(
                        bounds.left(),
                        bounds.bottom()
                            + gpui::px(if scrollbar.outside {
                                0.0
                            } else {
                                -track_height
                            }),
                    ),
                    gpui::size(bounds.size.width, gpui::px(track_height)),
                );
                let hitbox = if scrollbar.visible {
                    Some(window.insert_hitbox(
                        track.intersect(&window.content_mask().bounds),
                        gpui::HitboxBehavior::Normal,
                    ))
                } else {
                    None
                };
                Some((track, hitbox, maximum, length, start))
            },
            move |_, geometry, window, cx| {
                let Some((track, hitbox, maximum, length, start)) = geometry else {
                    if scrollbar.enabled {
                        grab.set(None);
                        dragging.set(false);
                    }
                    return;
                };
                if scrollbar.visible {
                    let mut color = scrollbar
                        .color
                        .map(|value| gpui::rgba(value).into())
                        .unwrap_or_else(|| {
                            let mut color = cx.theme().foreground;
                            color.a *= 0.20;
                            color
                        });
                    if scrollbar.outside {
                        color.a = if hovered.get() { 0.35 } else { 0.20 };
                    }
                    let inset = if scrollbar.overlay && !scrollbar.outside {
                        1.0
                    } else {
                        2.0
                    };
                    let thumb = gpui::Bounds::new(
                        gpui::point(
                            track.left() + gpui::px(start + inset),
                            track.top() + gpui::px(inset),
                        ),
                        gpui::size(
                            gpui::px((length - 2.0 * inset).max(0.0)),
                            gpui::px(track_height - 2.0 * inset),
                        ),
                    );
                    window.paint_quad(
                        gpui::fill(thumb, color).corner_radii(gpui::px(if scrollbar.outside {
                            3.0
                        } else {
                            2.0
                        })),
                    );
                }
                let down_grab = grab.clone();
                let down_dragging = dragging.clone();
                let down_binding = paint_binding.clone();
                window.on_mouse_event(move |event: &gpui::MouseDownEvent, phase, window, cx| {
                    if !phase.bubble()
                        || event.button != gpui::MouseButton::Left
                        || !track.contains(&event.position)
                        || !hitbox
                            .as_ref()
                            .is_some_and(|hitbox| hitbox.is_hovered(window))
                    {
                        return;
                    }
                    let x = f32::from(event.position.x - track.left());
                    if x >= start && x <= start + length {
                        down_grab.set(Some(x - start));
                        down_dragging.set(true);
                    } else {
                        let mut group = down_binding.group.borrow_mut();
                        let pane = &group.panes[&down_binding.pane];
                        let delta = if x < start {
                            -pane.viewport
                        } else {
                            pane.viewport
                        };
                        let target = pane.left() + delta;
                        if group.scroll_to(&down_binding.pane, target) {
                            cx.notify(down_binding.view);
                        }
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                });
                let move_grab = grab.clone();
                let move_binding = paint_binding.clone();
                let move_hovered = hovered.clone();
                window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, _, cx| {
                    if !phase.capture() {
                        return;
                    }
                    if scrollbar.outside {
                        let on_thumb = track.contains(&event.position)
                            && f32::from(event.position.x - track.left()) >= start
                            && f32::from(event.position.x - track.left()) <= start + length;
                        if move_hovered.replace(on_thumb) != on_thumb {
                            cx.notify(move_binding.view);
                        }
                    }
                    let Some(offset) = move_grab.get() else {
                        return;
                    };
                    let travel = f32::from(track.size.width) - length;
                    let x = f32::from(event.position.x - track.left()) - offset;
                    let target = if travel > 0.0 {
                        (x / travel).clamp(0.0, 1.0) * maximum
                    } else {
                        0.0
                    };
                    if move_binding
                        .group
                        .borrow_mut()
                        .scroll_to(&move_binding.pane, target)
                    {
                        cx.notify(move_binding.view);
                    }
                    cx.stop_propagation();
                });
                let up_grab = grab.clone();
                let up_dragging = dragging.clone();
                window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, _, cx| {
                    if phase.capture()
                        && event.button == gpui::MouseButton::Left
                        && up_grab.replace(None).is_some()
                    {
                        up_dragging.set(false);
                        cx.stop_propagation();
                    }
                });
            },
        )
        .absolute()
        .inset_0();
        let wheel = self.clone();
        let base = if scrollbar.outside {
            base
        } else {
            base.overflow_hidden()
        };
        let base = base
            .relative()
            .on_scroll_wheel(move |event, window, cx| wheel.wheel(event, window, cx));
        if scrollbar.only {
            // A common scrollbar must not replace the code viewport's ScrollHandle
            // bounds: Find uses those bounds to reveal text in the shaped row.
            base.child(observer).into_any_element()
        } else {
            let padded = gpui::div()
                .w(gpui::px(extent))
                .min_w_full()
                .flex_shrink_0()
                .pt(gpui::px(scrollbar.content_padding[0]))
                .pr(gpui::px(scrollbar.content_padding[1]))
                .pb(gpui::px(scrollbar.content_padding[2]))
                .pl(gpui::px(scrollbar.content_padding[3]))
                .child(content);
            let area = gpui::div()
                .id(format!("{id}-horizontal-area"))
                .overflow_hidden()
                .track_scroll(&handle);
            if scrollbar.overlay && !scrollbar.outside {
                base.child(area.size_full().child(padded.h_full()))
                    .child(observer)
                    .into_any_element()
            } else {
                let base = base
                    .flex()
                    .flex_col()
                    .child(area.w_full().flex_shrink_0().child(padded));
                let base = if scrollbar.enabled && !scrollbar.overlay && overflowing {
                    base.child(
                        gpui::div()
                            .w_full()
                            .h(gpui::px(track_height))
                            .flex_shrink_0(),
                    )
                } else {
                    base
                };
                base.child(observer).into_any_element()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn group() -> Group {
        let make = |extent| Pane {
            extent,
            viewport: 400.0,
            seen: true,
            ..Pane::default()
        };
        Group {
            revision: 0,
            position: 0.0,
            runtime: std::sync::Arc::new(crate::runtime::RuntimeState::new()),
            window_id: 1,
            panes: HashMap::from([("old".into(), make(3000.0)), ("new".into(), make(600.0))]),
        }
    }
    #[test]
    fn split_sync_clamps_each_pane_without_echoing_a_short_panes_limit() {
        let mut group = group();
        assert!(group.scroll_to("old", 1000.0));
        assert_eq!(group.panes["old"].left(), 1000.0);
        assert_eq!(group.panes["new"].left(), 200.0);
        assert!(!group.scroll_to("new", 240.0));
        assert_eq!(group.panes["old"].left(), 1000.0);
        assert!(group.scroll_to("new", 160.0));
        assert_eq!(group.panes["old"].left(), 160.0);
    }
    #[test]
    fn scrollbar_handle_changes_drive_the_other_pane_once() {
        let mut group = group();
        group.scroll_to("old", 1000.0);
        group.panes["new"]
            .handle
            .set_offset(gpui::point(gpui::px(-80.0), gpui::px(0.0)));
        assert!(group.sync_from_handle("new"));
        assert_eq!(group.panes["old"].left(), 80.0);
        assert!(!group.sync_from_handle("new"));
    }
}
