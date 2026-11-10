//! Link hit testing and source-span geometry without interactive overlay elements.
use super::{tabs::RichLayout, NativeRichRun};
use crate::gpui;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
struct Target {
    id: String,
    link: String,
    bounds: gpui::Bounds<gpui::Pixels>,
}
#[derive(Default)]
pub(super) struct State {
    event: Option<String>,
    target: Option<Target>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn observe(
    state: &Arc<Mutex<State>>,
    event: &str,
    element: &str,
    runs: &[NativeRichRun],
    layout: &RichLayout,
    point: gpui::Point<gpui::Pixels>,
    hovered: bool,
    runtime: &crate::SharedRuntime,
    window_id: u64,
) -> bool {
    let target = if hovered {
        layout.index_for_position(point).ok().and_then(|index| {
            runs.iter().enumerate().find_map(|(number, run)| {
                if !run.range.contains(&index) {
                    return None;
                }
                let link = run.source.link.as_ref()?;
                Some(Target {
                    id: format!("{element}:{number}"),
                    link: link.clone(),
                    bounds: layout.bounds_for_range(run.range.clone())?,
                })
            })
        })
    } else {
        None
    };
    let Ok(mut state) = state.lock() else {
        return false;
    };
    if state.target == target && state.event.as_deref() == Some(event) {
        return false;
    }
    let target_is_present = target.is_some();
    let reported = target.as_ref().or(state.target.as_ref());
    if let Some(target) = reported {
        let _ = crate::push_event(
            runtime,
            crate::NativeEvent::LinkHover {
                window_id,
                event: event.to_string(),
                link: Some(target.link.clone()),
                hovered: target_is_present,
                value: crate::event::ElementBoundsGeometry {
                    id: target.id.clone(),
                    x: f32::from(target.bounds.left()) as f64,
                    y: f32::from(target.bounds.top()) as f64,
                    width: f32::from(target.bounds.size.width) as f64,
                    height: f32::from(target.bounds.size.height) as f64,
                    coordinate_space: "window_native_pixels".into(),
                },
            },
        );
    }
    state.target = target;
    state.event = Some(event.to_string());
    true
}

#[cfg(all(test, feature = "native-test"))]
mod tests {
    use super::super::tabs::TabLayout;
    use super::*;

    #[gpui::test]
    fn link_hover_tracks_unicode_tabs_geometry_changes_and_exit(cx: &mut gpui::TestAppContext) {
        let cx = cx.add_empty_window();
        let runtime = Arc::new(crate::runtime::RuntimeState::new());
        let state = Arc::new(Mutex::new(State::default()));
        let text = "🌱\t@Mira";
        let layout = cx.update(|window, _cx| {
            let layout = TabLayout::new(
                text.into(),
                vec![window.text_style().to_run(text.len())],
                4,
                window,
            );
            layout.prepaint(gpui::Bounds::new(
                gpui::point(gpui::px(50.), gpui::px(30.)),
                gpui::size(gpui::px(400.), gpui::px(30.)),
            ));
            layout
        });
        let mut run = crate::RichTextRunNode {
            range: crate::TextRange {
                start: crate::TextPosition {
                    line: 0,
                    utf16_offset: 3,
                },
                end: crate::TextPosition {
                    line: 0,
                    utf16_offset: 8,
                },
            },
            color: None,
            background: None,
            background_opacity: None,
            font_family: None,
            font_weight: None,
            font_style: None,
            underline: None,
            underline_style: None,
            strikethrough: None,
            link: Some("@mira".into()),
        };
        let runs = vec![NativeRichRun {
            range: 5..10,
            source: run.clone(),
        }];
        let rich = RichLayout::Tabbed(layout.clone());
        let bounds = rich.bounds_for_range(5..10).unwrap();
        assert!(bounds.origin.x > gpui::px(50.));
        assert!(bounds.size.width > gpui::px(10.));
        assert!(observe(
            &state,
            "hover",
            "message",
            &runs,
            &rich,
            bounds.center(),
            true,
            &runtime,
            7
        ));
        assert!(!observe(
            &state,
            "hover",
            "message",
            &runs,
            &rich,
            bounds.center(),
            true,
            &runtime,
            7
        ));
        layout.prepaint(gpui::Bounds::new(
            gpui::point(gpui::px(100.), gpui::px(70.)),
            gpui::size(gpui::px(400.), gpui::px(30.)),
        ));
        let moved = rich.bounds_for_range(5..10).unwrap();
        assert!(observe(
            &state,
            "hover",
            "message",
            &runs,
            &rich,
            moved.center(),
            true,
            &runtime,
            7
        ));
        assert!(observe(
            &state,
            "hover",
            "message",
            &runs,
            &rich,
            moved.center(),
            false,
            &runtime,
            7
        ));
        let events = runtime.events.lock().unwrap();
        assert_eq!(events.len(), 3);
        assert!(
            matches!(&events[0], crate::NativeEvent::LinkHover { window_id: 7, hovered: true, link: Some(link), value, .. } if link == "@mira" && value.id == "message:0")
        );
        assert!(
            matches!(&events[1], crate::NativeEvent::LinkHover { hovered: true, value, .. } if value.y == 70.)
        );
        assert!(matches!(
            &events[2],
            crate::NativeEvent::LinkHover { hovered: false, .. }
        ));
        // Unlinked formatting spans must not become hover targets.
        drop(events);
        run.link = None;
        let runs = vec![NativeRichRun {
            range: 5..10,
            source: run,
        }];
        assert!(!observe(
            &state,
            "hover",
            "message",
            &runs,
            &rich,
            moved.center(),
            true,
            &runtime,
            7
        ));
    }
}
