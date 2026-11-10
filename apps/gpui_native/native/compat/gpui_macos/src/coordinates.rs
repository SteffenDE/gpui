//! AppKit and GPUI share logical lengths but use opposite global y axes.
use cocoa::{
    appkit::NSScreen,
    base::nil,
    foundation::{NSArray, NSPoint, NSRect},
};
use gpui::{Bounds, Pixels, point, px, size};

pub(crate) fn primary_screen_top() -> f64 {
    unsafe {
        let screen = NSArray::objectAtIndex(NSScreen::screens(nil), 0);
        let frame = NSScreen::frame(screen);
        frame.origin.y + frame.size.height
    }
}

pub(crate) fn global_bounds(frame: NSRect, primary_top: f64) -> Bounds<Pixels> {
    Bounds::new(
        point(
            px(frame.origin.x as f32),
            px((primary_top - frame.origin.y - frame.size.height) as f32),
        ),
        size(px(frame.size.width as f32), px(frame.size.height as f32)),
    )
}

pub(crate) fn appkit_top_left(bounds: Bounds<Pixels>, primary_top: f64) -> NSPoint {
    NSPoint::new(
        f32::from(bounds.origin.x) as f64,
        primary_top - f32::from(bounds.origin.y) as f64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cocoa::foundation::NSSize;

    #[test]
    fn global_frames_round_trip_for_every_display_direction() {
        for (x, y) in [
            (0.0, 0.0),
            (1920.0, 80.0),
            (-1440.0, 120.0),
            (150.0, -900.0),
            (200.0, 1080.0),
        ] {
            let frame = NSRect::new(
                NSPoint::new(x + 20.5, 1080.0 - y - 700.0),
                NSSize::new(1000.0, 700.0),
            );
            let bounds = global_bounds(frame, 1080.0);
            assert_eq!(f32::from(bounds.origin.x), (x + 20.5) as f32);
            assert_eq!(f32::from(bounds.origin.y), y as f32);
            let top_left = appkit_top_left(bounds, 1080.0);
            assert_eq!(top_left.x, frame.origin.x);
            assert_eq!(top_left.y, frame.origin.y + frame.size.height);
        }
    }

    #[test]
    fn secondary_work_area_preserves_global_origin_and_insets() {
        let visible = NSRect::new(NSPoint::new(-1600.0, -150.0), NSSize::new(1600.0, 850.0));
        let bounds = global_bounds(visible, 1080.0);
        assert_eq!(
            bounds,
            Bounds::new(point(px(-1600.0), px(380.0)), size(px(1600.0), px(850.0)))
        );
    }
}
