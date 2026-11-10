//! Single-line ellipsis preserving shaped clusters and full accessible labels.
use crate::gpui;
use std::{cell::RefCell, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;

pub(super) struct ElidedText {
    source: gpui::SharedString,
    start: bool,
    layout: Option<Rc<RefCell<Layout>>>,
}
struct Layout {
    full: gpui::ShapedLine,
    ellipsis: gpui::ShapedLine,
    suffix: gpui::ShapedLine,
    cuts: Vec<usize>,
    truncated: bool,
    height: gpui::Pixels,
}
impl ElidedText {
    pub(super) fn new(source: String) -> Self {
        Self {
            source: source.into(),
            start: true,
            layout: None,
        }
    }
}
impl ElidedText {
    pub(super) fn end(source: String) -> Self {
        Self {
            start: false,
            ..Self::new(source)
        }
    }
}
impl gpui::IntoElement for ElidedText {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl gpui::Element for ElidedText {
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
        let height = window.pixel_snap(style.line_height.to_pixels(size.into(), window.rem_size()));
        let full = window.text_system().shape_line(
            self.source.clone(),
            size,
            &[style.to_run(self.source.len())],
            None,
        );
        let ellipsis =
            window
                .text_system()
                .shape_line("…".into(), size, &[style.to_run("…".len())], None);
        let graphemes = self
            .source
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .collect::<std::collections::HashSet<_>>();
        // A grapheme may share a ligature glyph with its neighbors. Only cut
        // where both the grapheme and native shaped cluster begin.
        let mut cuts = full
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .map(|glyph| glyph.index)
            .filter(|index| graphemes.contains(index))
            .collect::<Vec<_>>();
        cuts.push(self.source.len());
        cuts.sort_unstable();
        cuts.dedup();
        let layout = Rc::new(RefCell::new(Layout {
            suffix: full.clone(),
            full,
            ellipsis,
            cuts,
            truncated: false,
            height,
        }));
        self.layout = Some(layout.clone());
        let start = self.start;
        let measure = move |known: gpui::Size<Option<gpui::Pixels>>,
                            available: gpui::Size<gpui::AvailableSpace>,
                            _: &mut gpui::Window,
                            _: &mut gpui::App| {
            let mut layout = layout.borrow_mut();
            let full_width = layout.full.width;
            let width = known
                .width
                .unwrap_or_else(|| match available.width {
                    gpui::AvailableSpace::Definite(width) => width.min(full_width),
                    gpui::AvailableSpace::MinContent => gpui::px(0.0),
                    _ => full_width,
                })
                .max(gpui::px(0.0));
            layout.truncated = full_width > width;
            if layout.truncated {
                if start {
                    let cut = layout
                        .cuts
                        .iter()
                        .copied()
                        .find(|index| {
                            full_width - layout.full.x_for_index(*index) + layout.ellipsis.width
                                <= width
                        })
                        .unwrap_or(layout.full.len());
                    layout.suffix = layout.full.split_at(cut).1;
                } else {
                    let cut = layout
                        .cuts
                        .iter()
                        .rev()
                        .copied()
                        .find(|index| {
                            layout.full.x_for_index(*index) + layout.ellipsis.width <= width
                        })
                        .unwrap_or(0);
                    layout.suffix = layout.full.split_at(cut).0;
                }
            } else {
                layout.suffix = layout.full.clone();
            }
            gpui::size(width, known.height.unwrap_or(layout.height))
        };
        let id = if start {
            window.request_measured_layout(Default::default(), measure)
        } else {
            window.request_precise_measured_layout(
                gpui::Style {
                    preserve_layout_bounds: true,
                    ..Default::default()
                },
                measure,
            )
        };
        (id, ())
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
        let Some(layout) = &self.layout else { return };
        let layout = layout.borrow();
        let width = layout.suffix.width
            + if layout.truncated {
                layout.ellipsis.width
            } else {
                gpui::px(0.0)
            };
        let mut origin = gpui::point(
            if self.start {
                bounds.right() - width
            } else {
                bounds.left()
            },
            bounds.top(),
        );
        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            if layout.truncated && self.start {
                let _ = layout.ellipsis.paint(
                    origin,
                    layout.height,
                    gpui::TextAlign::Left,
                    None,
                    window,
                    cx,
                );
                origin.x += layout.ellipsis.width;
            }
            let _ = layout.suffix.paint(
                origin,
                layout.height,
                gpui::TextAlign::Left,
                None,
                window,
                cx,
            );
            if layout.truncated && !self.start {
                origin.x += layout.suffix.width;
                let _ = layout.ellipsis.paint(
                    origin,
                    layout.height,
                    gpui::TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
        });
    }
}
