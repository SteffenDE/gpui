//! Popovers anchored to measured content such as a rich-text link span.
use super::super::super::{component_registry::PopoverHoverState, controlled::SharedBinding};
use crate::{gpui, ElementRenderContext, PopoverComponentNode};
use std::sync::{Arc, Mutex};

#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    node: &PopoverComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
    content: gpui::Stateful<gpui::Div>,
    open: bool,
    binding: &SharedBinding<bool>,
    effective: &Arc<Mutex<bool>>,
    hover: &Arc<Mutex<PopoverHoverState>>,
) -> Option<gpui::AnyElement> {
    use gpui::{InteractiveElement, IntoElement, ParentElement, Styled};
    use gpui_component::StyledExt;
    let (x, y, width, height) = (
        node.anchor_x?,
        node.anchor_y?,
        node.anchor_width?,
        node.anchor_height?,
    );
    let rectangle = gpui::Bounds::new(
        gpui::point(gpui::px(x as f32), gpui::px(y as f32)),
        gpui::size(gpui::px(width as f32), gpui::px(height as f32)),
    );
    let runtime = context.runtime.clone();
    let window_id = context.window_id;
    let parent = context.window.current_view();
    super::request_hover(
        &runtime,
        window_id,
        Some(parent),
        binding,
        effective,
        hover,
        true,
        node.anchor_hovered,
        node.hover_delay,
        node.close_delay,
        context.cx,
    );
    let mut root = gpui::div()
        .id(format!("{}-virtual-anchor", node.id))
        .absolute()
        .w_0()
        .h_0();
    if !open {
        return Some(root.into_any_element());
    }
    let mut panel = gpui::div().flex().flex_col().occlude().tab_group();
    if node.appearance {
        panel = panel.popover_style(context.cx).p_3();
    }
    let mut panel = crate::apply_generated_render_styles(panel, node.style.clone())
        .id(format!("{}-virtual-panel", node.id))
        .child(content);
    if node.closable {
        let binding = binding.clone();
        let effective = effective.clone();
        let hover = hover.clone();
        let popover_id = node.id.clone();
        panel = panel.on_mouse_down_out(move |event, _window, cx| {
            if runtime.inside_popover_descendant(window_id, &popover_id, event.position) {
                return;
            }
            if let Ok(mut state) = hover.lock() {
                state.generation = state.generation.wrapping_add(1);
            }
            super::request_open(&runtime, window_id, &binding, &effective, false, true);
            cx.notify(parent);
        });
    }
    let anchor = super::anchor(node.anchor.as_deref());
    let point = match anchor {
        gpui::Anchor::TopLeft => rectangle.bottom_left() + gpui::point(gpui::px(0.), gpui::px(4.)),
        gpui::Anchor::TopCenter => {
            rectangle.bottom_center() + gpui::point(gpui::px(0.), gpui::px(4.))
        }
        gpui::Anchor::TopRight => {
            rectangle.bottom_right() + gpui::point(gpui::px(0.), gpui::px(4.))
        }
        gpui::Anchor::BottomLeft => rectangle.origin - gpui::point(gpui::px(0.), gpui::px(4.)),
        gpui::Anchor::BottomCenter => {
            rectangle.top_center() - gpui::point(gpui::px(0.), gpui::px(4.))
        }
        gpui::Anchor::BottomRight => {
            rectangle.top_right() - gpui::point(gpui::px(0.), gpui::px(4.))
        }
        gpui::Anchor::LeftCenter => {
            rectangle.right_center() + gpui::point(gpui::px(4.), gpui::px(0.))
        }
        gpui::Anchor::RightCenter => {
            rectangle.left_center() - gpui::point(gpui::px(4.), gpui::px(0.))
        }
    };
    root = root.child(
        gpui::deferred(
            gpui::anchored()
                .anchor(anchor)
                .position(point)
                .snap_to_window_with_margin(gpui::px(8.))
                .child(panel),
        )
        .with_priority(1),
    );
    Some(root.into_any_element())
}
