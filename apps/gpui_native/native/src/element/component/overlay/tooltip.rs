use super::{invalid_slots, render_slot};
use crate::{
    gpui, ElementRenderContext, TooltipComponentNode, TooltipContentComponentNode,
    TooltipShortcutComponentNode, TooltipTriggerComponentNode,
};

#[cfg(feature = "components")]
#[derive(Clone, Debug)]
struct StyledTooltip {
    content: TooltipContentComponentNode,
    shortcut: Option<TooltipShortcutComponentNode>,
}

#[cfg(feature = "components")]
struct TooltipView {
    body: StyledTooltip,
    root: gpui::WeakEntity<crate::ElixirRoot>,
    namespace: String,
}

#[cfg(feature = "components")]
impl gpui::Render for TooltipView {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement, Styled};
        let mut body = crate::apply_generated_render_styles(
            gpui::div().flex().items_center(),
            self.body.content.style.clone(),
        );
        if self.body.content.children.is_empty() {
            body = body.child(gpui::SharedString::from(self.body.content.text.clone()));
        } else if let Some(root) = self.root.upgrade() {
            let nodes = self.body.content.children.clone();
            let children = root.update(cx, |root, cx| {
                root.render_tooltip_children(nodes, self.namespace.clone(), window, cx)
            });
            body = body.children(children);
        }
        if let Some(shortcut) = self.body.shortcut.clone() {
            body = body.child(render_tooltip_shortcut(shortcut));
        }
        body
    }
}

#[cfg(feature = "components")]
fn presentation_revision(text: &str, appearance: bool, body: &Option<StyledTooltip>) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hash = DefaultHasher::new();
    text.hash(&mut hash);
    appearance.hash(&mut hash);
    format!("{body:?}").hash(&mut hash);
    hash.finish()
}

#[cfg(feature = "components")]
pub(crate) fn render_tooltip(
    node: TooltipComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    use gpui::{
        AppContext, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    };
    use gpui_component::tooltip::Tooltip;
    use std::time::Duration;
    let mut trigger = None;
    let mut content = None;
    let mut shortcut = None;
    for child in node.children {
        match child {
            crate::ElementNode::TooltipTriggerComponent(value) if trigger.is_none() => {
                trigger = Some(value)
            }
            crate::ElementNode::TooltipContentComponent(value) if content.is_none() => {
                content = Some(value)
            }
            crate::ElementNode::TooltipShortcutComponent(value) if shortcut.is_none() => {
                shortcut = Some(value)
            }
            _ => return invalid_slots(),
        }
    }
    let Some(trigger) = trigger else {
        return invalid_slots();
    };
    let body = content.map(|content| StyledTooltip { content, shortcut });
    let revision = presentation_revision(&node.text, node.appearance, &body);
    let text: gpui::SharedString = node.text.into();
    let appearance = node.appearance;
    let root = context.cx.entity().downgrade();
    let namespace = format!("{}-tooltip-{}", context.id_namespace, node.id);
    let build = move |window: &mut gpui::Window, cx: &mut gpui::App| {
        if let Some(body) = body.clone() {
            if !appearance {
                return cx
                    .new(|_| TooltipView {
                        body,
                        root: root.clone(),
                        namespace: namespace.clone(),
                    })
                    .into();
            }
            let root = root.clone();
            let namespace = namespace.clone();
            Tooltip::element(move |_window, cx| {
                cx.new(|_| TooltipView {
                    body: body.clone(),
                    root: root.clone(),
                    namespace: namespace.clone(),
                })
            })
            .build(window, cx)
        } else {
            Tooltip::new(text.clone()).build(window, cx)
        }
    };
    let inherited_focus = context.tooltip_focus_owner.clone();
    // A tooltip decorating a popover shares the actual trigger's focus owner.
    // Ordinary controls use a non-tabbable scope around their child trigger.
    let focus = if node.keyboard && inherited_focus.is_none() {
        Some(
            context
                .window
                .use_keyed_state(format!("{}-tooltip-focus", node.id), context.cx, |_, cx| {
                    cx.focus_handle()
                })
                .read(context.cx)
                .clone()
                .tab_stop(false),
        )
    } else {
        inherited_focus.clone().filter(|_| node.keyboard)
    };
    let mut element = crate::apply_generated_render_styles(gpui::div(), node.style)
        .id(node.id)
        .child(render_tooltip_trigger(trigger, context));
    if let Some(focus) = focus {
        if inherited_focus.is_none() {
            element = element
                .track_focus(&focus)
                .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                    window.prevent_default()
                });
        }
        element = element.tooltip_on_focus(&focus);
    }
    element = if node.hoverable {
        element.hoverable_tooltip(build)
    } else {
        element.tooltip(build)
    };
    let placement = match node.placement.as_deref() {
        Some("top") => Some(gpui::TooltipPlacement::Top),
        Some("top_start") => Some(gpui::TooltipPlacement::TopStart),
        Some("top_end") => Some(gpui::TooltipPlacement::TopEnd),
        Some("bottom_start") => Some(gpui::TooltipPlacement::BottomStart),
        Some("bottom_end") => Some(gpui::TooltipPlacement::BottomEnd),
        Some("bottom") => Some(gpui::TooltipPlacement::Bottom),
        Some("left") => Some(gpui::TooltipPlacement::Left),
        Some("right") => Some(gpui::TooltipPlacement::Right),
        _ => None,
    };
    if let Some(placement) = placement {
        element = element
            .tooltip_placement(placement, gpui::px(node.offset.max(0.) as f32))
            .tooltip_collision_padding(gpui::px(
                node.collision_padding.clamp(0., 1_000_000.) as f32
            ));
    }
    let delay = if node.delay.is_finite() {
        node.delay.clamp(0.0, 60_000.0)
    } else {
        500.0
    };
    let close_delay = if node.close_delay.is_finite() {
        node.close_delay.clamp(0., 60_000.)
    } else {
        500.
    };
    element
        .tooltip_revision(revision)
        .tooltip_show_delay(Duration::from_secs_f64(delay / 1000.0))
        .tooltip_hide_delay(Duration::from_secs_f64(close_delay / 1000.0))
        .into_any_element()
}

#[cfg(not(feature = "components"))]
pub(crate) fn render_tooltip(
    node: TooltipComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    match node.children.into_iter().find_map(|child| match child {
        crate::ElementNode::TooltipTriggerComponent(trigger) => Some(trigger),
        _ => None,
    }) {
        Some(trigger) => render_tooltip_trigger(trigger, context),
        None => invalid_slots(),
    }
}

pub(crate) fn render_tooltip_trigger(
    node: TooltipTriggerComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    render_slot(node.style, node.children, context)
}

pub(crate) fn render_tooltip_content(node: TooltipContentComponentNode) -> gpui::AnyElement {
    use gpui::{IntoElement, ParentElement};
    crate::apply_generated_render_styles(gpui::div(), node.style)
        .child(gpui::SharedString::from(node.text))
        .into_any_element()
}

pub(crate) fn render_tooltip_shortcut(node: TooltipShortcutComponentNode) -> gpui::AnyElement {
    use gpui::{IntoElement, ParentElement};
    let element = crate::apply_generated_render_styles(gpui::div(), node.style);
    if node.letter_spacing == 0.0 {
        element
            .child(gpui::SharedString::from(node.text))
            .into_any_element()
    } else {
        element
            .child(crate::element::tracked_text::TrackedText::new(
                node.text,
                node.letter_spacing as f32,
            ))
            .into_any_element()
    }
}

#[cfg(all(test, feature = "components"))]
mod tests {
    use crate::gpui::{point, px, size, Bounds, TooltipAnchor, TooltipPlacement};
    #[test]
    fn tooltip_revision_tracks_shortcut_spacing() {
        use super::{presentation_revision, StyledTooltip};
        use crate::{StyleAttrs, TooltipContentComponentNode, TooltipShortcutComponentNode};
        let mut body = StyledTooltip {
            content: TooltipContentComponentNode {
                style: StyleAttrs::default(),
                children: vec![],
                text: "Search".into(),
            },
            shortcut: Some(TooltipShortcutComponentNode {
                style: StyleAttrs::default(),
                text: "⌘⇧F".into(),
                letter_spacing: 1.2,
            }),
        };
        let initial = presentation_revision("Search", false, &Some(body.clone()));
        body.shortcut.as_mut().unwrap().letter_spacing = 1.8;
        assert_ne!(initial, presentation_revision("Search", false, &Some(body)));
    }

    #[test]
    fn tooltip_revision_tracks_text_style_and_appearance() {
        use super::{presentation_revision, StyledTooltip};
        use crate::{StyleAttrs, TooltipContentComponentNode};
        let body = StyledTooltip {
            content: TooltipContentComponentNode {
                style: StyleAttrs::default(),
                children: vec![],
                text: "Initial".into(),
            },
            shortcut: None,
        };
        let baseline = presentation_revision("Label", false, &Some(body.clone()));
        assert_eq!(
            baseline,
            presentation_revision("Label", false, &Some(body.clone()))
        );
        assert_ne!(
            baseline,
            presentation_revision("Changed", false, &Some(body.clone()))
        );
        assert_ne!(
            baseline,
            presentation_revision("Label", true, &Some(body.clone()))
        );
        let mut updated = body.clone();
        updated.content.text = "Updated".into();
        assert_ne!(
            baseline,
            presentation_revision("Label", false, &Some(updated))
        );
        let mut updated = body;
        updated.content.style.background = Some(0x273348);
        assert_ne!(
            baseline,
            presentation_revision("Label", false, &Some(updated))
        );
    }

    #[test]
    fn anchored_tooltips_center_flip_and_clamp() {
        let anchor = TooltipAnchor {
            bounds: Bounds::new(point(px(100.), px(60.)), size(px(24.), px(24.))),
            placement: TooltipPlacement::Top,
            offset: px(4.),
        };
        let tip = size(px(104.), px(28.));
        let viewport = size(px(400.), px(200.));
        assert_eq!(
            anchor.tooltip_bounds(tip, viewport, px(4.)).origin,
            point(px(60.), px(28.))
        );
        let near_top = TooltipAnchor {
            bounds: Bounds::new(point(px(0.), px(2.)), size(px(24.), px(24.))),
            ..anchor
        };
        assert_eq!(
            near_top.tooltip_bounds(tip, viewport, px(4.)).origin,
            point(px(4.), px(30.))
        );
        let near_right = TooltipAnchor {
            bounds: Bounds::new(point(px(380.), px(60.)), size(px(24.), px(24.))),
            placement: TooltipPlacement::Right,
            ..anchor
        };
        assert_eq!(
            near_right.tooltip_bounds(tip, viewport, px(4.)).origin,
            point(px(272.), px(58.))
        );
        assert_eq!(
            anchor
                .tooltip_bounds(size(px(900.), px(900.)), viewport, px(4.))
                .origin,
            point(px(4.), px(4.))
        );
    }
}
