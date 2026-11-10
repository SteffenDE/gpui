//! Single-line shortcut shaping with grapheme-aware tracking.
use crate::gpui;
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

pub(super) struct TrackedText {
    source: gpui::SharedString,
    spacing: gpui::Pixels,
    line: Option<gpui::ShapedLine>,
    height: gpui::Pixels,
}

impl TrackedText {
    pub(super) fn new(source: String, spacing: f32) -> Self {
        Self {
            source: source.into(),
            spacing: gpui::px(if spacing.is_finite() { spacing } else { 0.0 }),
            line: None,
            height: gpui::px(0.0),
        }
    }
}

fn tracked_line(mut line: gpui::ShapedLine, spacing: gpui::Pixels) -> gpui::ShapedLine {
    if spacing == gpui::px(0.0) || line.text.is_empty() {
        return line;
    }
    let boundaries: Vec<_> = line
        .text
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .collect();
    let cluster = |index: usize| {
        boundaries
            .partition_point(|boundary| *boundary <= index)
            .saturating_sub(1)
    };
    let mut visual = boundaries
        .iter()
        .enumerate()
        .map(|(ordinal, index)| {
            let x = line
                .runs
                .iter()
                .flat_map(|run| &run.glyphs)
                .filter(|glyph| cluster(glyph.index) == ordinal)
                .map(|glyph| glyph.position.x)
                .min_by(|a, b| f32::from(*a).total_cmp(&f32::from(*b)))
                .unwrap_or_else(|| line.x_for_index(*index));
            (ordinal, x)
        })
        .collect::<Vec<_>>();
    visual.sort_by(|a, b| {
        f32::from(a.1)
            .total_cmp(&f32::from(b.1))
            .then(a.0.cmp(&b.0))
    });
    let mut offsets = vec![gpui::px(0.0); boundaries.len()];
    for (order, (ordinal, _)) in visual.into_iter().enumerate() {
        offsets[ordinal] = spacing * order as f32;
    }
    let mut runs = line.runs.clone();
    for glyph in runs.iter_mut().flat_map(|run| &mut run.glyphs) {
        glyph.position.x += offsets[cluster(glyph.index)];
    }
    let layout = gpui::LineLayout {
        font_size: line.font_size,
        width: (line.width + spacing * boundaries.len() as f32).max(gpui::px(0.0)),
        ascent: line.ascent,
        descent: line.descent,
        runs,
        len: line.len(),
    };
    *line = Arc::new(layout);
    line
}

impl gpui::IntoElement for TrackedText {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for TrackedText {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> (gpui::LayoutId, ()) {
        let style = window.text_style();
        let size = style.font_size.to_pixels(window.rem_size());
        self.height = style.line_height.to_pixels(size.into(), window.rem_size());
        let mut run = style.to_run(self.source.len());
        // Zero-spacing precise labels keep the original font features.
        // Nonzero tracking separates typographic letters rather than ligatures.
        if self.spacing != gpui::px(0.0) {
            let mut features = run.font.features.tag_value_list().to_vec();
            features.retain(|(tag, _)| tag != "liga" && tag != "clig");
            features.extend([("liga".into(), 0), ("clig".into(), 0)]);
            run.font.features = gpui::FontFeatures(Arc::new(features));
        }
        let line = window
            .text_system()
            .shape_line(self.source.clone(), size, &[run], None);
        let line = tracked_line(line, self.spacing);
        let natural = gpui::size(line.width, self.height);
        self.line = Some(line);
        let layout = window.request_precise_measured_layout(
            gpui::Style {
                preserve_layout_bounds: true,
                ..Default::default()
            },
            move |known, _, _, _| {
                gpui::size(
                    known.width.unwrap_or(natural.width),
                    known.height.unwrap_or(natural.height),
                )
            },
        );
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        _: &mut (),
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) {
        if let Some(line) = &self.line {
            let _ = line.paint(
                bounds.origin,
                self.height,
                gpui::TextAlign::Left,
                None,
                window,
                cx,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, positions: &[(usize, f32)]) -> gpui::ShapedLine {
        let mut line = gpui::ShapedLine::default();
        line.text = text.to_string().into();
        *line = Arc::new(gpui::LineLayout {
            font_size: gpui::px(12.),
            width: gpui::px(18.),
            ascent: gpui::px(9.),
            descent: gpui::px(3.),
            len: text.len(),
            runs: vec![gpui::ShapedRun {
                font_id: gpui::FontId(0),
                glyphs: positions
                    .iter()
                    .map(|(index, x)| gpui::ShapedGlyph {
                        id: gpui::GlyphId(1),
                        position: gpui::point(gpui::px(*x), gpui::px(0.)),
                        index: *index,
                        is_emoji: false,
                    })
                    .collect(),
            }],
        });
        line
    }

    #[test]
    fn tracking_moves_each_shortcut_glyph_and_includes_the_final_advance() {
        let original = line("⌘⇧F", &[(0, 0.), (3, 6.), (6, 12.)]);
        let result = tracked_line(original.clone(), gpui::px(1.2));
        let positions: Vec<_> = result.runs[0]
            .glyphs
            .iter()
            .map(|g| f32::from(g.position.x))
            .collect();
        assert_eq!(positions, vec![0., 7.2, 14.4]);
        assert!((f32::from(result.width) - 21.6).abs() < 0.001);
        assert_eq!(original.runs[0].glyphs[1].position.x, gpui::px(6.));
        assert_eq!(original.width, gpui::px(18.));
        let zero = tracked_line(original.clone(), gpui::px(0.));
        assert!(Arc::ptr_eq(&original, &zero));
    }

    #[test]
    fn tracking_keeps_combining_marks_with_the_base_and_follows_visual_order() {
        let result = tracked_line(
            line("e\u{301}F", &[(0, 0.), (1, 0.), (3, 6.)]),
            gpui::px(1.2),
        );
        assert_eq!(result.runs[0].glyphs[0].position.x, gpui::px(0.));
        assert_eq!(result.runs[0].glyphs[1].position.x, gpui::px(0.));
        assert_eq!(result.runs[0].glyphs[2].position.x, gpui::px(7.2));
        assert!((f32::from(result.width) - 20.4).abs() < 0.001);
        let result = tracked_line(line("אב", &[(0, 6.), (2, 0.)]), gpui::px(1.2));
        assert_eq!(result.runs[0].glyphs[0].position.x, gpui::px(7.2));
        assert_eq!(result.runs[0].glyphs[1].position.x, gpui::px(0.));
    }
}
