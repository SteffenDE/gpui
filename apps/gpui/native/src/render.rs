use zed_gpui as gpui;

pub fn missing_resource_placeholder() -> gpui::AnyElement {
    use gpui::{div, IntoElement, ParentElement, Styled};

    div()
        .flex()
        .items_center()
        .justify_center()
        .border(gpui::px(1.0))
        .border_color(gpui::rgb(0xcc3333))
        .bg(gpui::rgb(0x332222))
        .text_color(gpui::rgb(0xffaaaa))
        .p(gpui::px(8.0))
        .child("missing resource")
        .into_any_element()
}

pub fn motion_easing(easing: &str, delta: f32) -> f32 {
    match easing {
        "ease_in" => delta * delta * delta,
        "ease_in_out" => css_ease_in_out(delta),
        "ease_out" => 1.0 - (1.0 - delta).powi(3),
        _ => delta,
    }
}

// CSS ease-in-out: cubic-bezier(0.42, 0, 0.58, 1). Invert the
// Bezier's x coordinate before evaluating y so time follows CSS timing.
fn css_ease_in_out(delta: f32) -> f32 {
    if delta <= 0.0 || delta >= 1.0 {
        return delta.clamp(0.0, 1.0);
    }
    let (mut low, mut high): (f32, f32) = (0.0, 1.0);
    for _ in 0..20 {
        let t = (low + high) * 0.5;
        let x = 3.0 * (1.0 - t).powi(2) * t * 0.42 + 3.0 * (1.0 - t) * t * t * 0.58 + t * t * t;
        if x < delta {
            low = t;
        } else {
            high = t;
        }
    }
    let t = (low + high) * 0.5;
    3.0 * t * t - 2.0 * t * t * t
}

pub fn lerp(from: f32, to: f32, delta: f32) -> f32 {
    from + (to - from) * delta
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn css_ease_in_out_matches_css_quarter_points() {
        assert!((motion_easing("ease_in_out", 0.25) - 0.1291619).abs() < 0.00001);
        assert!((motion_easing("ease_in_out", 0.75) - 0.8708381).abs() < 0.00001);
    }

    #[test]
    fn motion_helpers_reach_declared_endpoints() {
        for easing in ["linear", "ease_in", "ease_in_out", "ease_out"] {
            assert_eq!(motion_easing(easing, 0.0), 0.0);
            assert_eq!(motion_easing(easing, 1.0), 1.0);
        }
        assert_eq!(lerp(4.0, 8.0, 0.0), 4.0);
        assert_eq!(lerp(4.0, 8.0, 1.0), 8.0);
    }
}
