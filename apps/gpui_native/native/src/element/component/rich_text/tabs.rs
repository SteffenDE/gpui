//! Tab stops with source-preserving byte indices and shaped glyph advances.
use crate::gpui;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub(super) enum RichLayout {
    Plain(gpui::TextLayout),
    Tabbed(TabLayout),
}
impl RichLayout {
    pub(super) fn position_for_index(&self, index: usize) -> Option<gpui::Point<gpui::Pixels>> {
        match self {
            Self::Plain(layout) => layout.position_for_index(index),
            Self::Tabbed(layout) => layout.position_for_index(index),
        }
    }
    pub(super) fn index_for_position(
        &self,
        point: gpui::Point<gpui::Pixels>,
    ) -> Result<usize, usize> {
        match self {
            Self::Plain(layout) => layout.index_for_position(point),
            Self::Tabbed(layout) => layout.index_for_position(point),
        }
    }
    pub(super) fn bounds_for_range(
        &self,
        range: std::ops::Range<usize>,
    ) -> Option<gpui::Bounds<gpui::Pixels>> {
        self.rectangles_for_range(range, gpui::TextAlign::Left)
            .into_iter()
            .reduce(|bounds, rectangle| bounds.union(&rectangle))
    }
    pub(super) fn rectangles_for_range(
        &self,
        range: std::ops::Range<usize>,
        align: gpui::TextAlign,
    ) -> Vec<gpui::Bounds<gpui::Pixels>> {
        let (bounds, height, lines) = match self {
            Self::Plain(layout) => (
                layout.bounds(),
                layout.line_height(),
                layout.line_layouts().into_iter().collect::<Vec<_>>(),
            ),
            Self::Tabbed(layout) => {
                let state = layout.0.borrow();
                (
                    state.bounds,
                    state.line_height,
                    state
                        .lines
                        .iter()
                        .map(|line| std::ops::Deref::deref(line).clone())
                        .collect(),
                )
            }
        };
        let origin = bounds.origin;
        let mut first = 0;
        let mut y = origin.y;
        let mut result = Vec::new();
        for line in lines {
            let base = &line.unwrapped_layout;
            let mut starts = vec![0];
            starts.extend(
                line.wrap_boundaries
                    .iter()
                    .map(|boundary| base.runs[boundary.run_ix].glyphs[boundary.glyph_ix].index),
            );
            for (row, start) in starts.iter().copied().enumerate() {
                let end = starts.get(row + 1).copied().unwrap_or(base.len);
                let a = range.start.saturating_sub(first).max(start);
                let b = range.end.saturating_sub(first).min(end);
                if a >= b {
                    continue;
                }
                let width = base.x_for_index(end) - base.x_for_index(start);
                let alignment = match align {
                    gpui::TextAlign::Left => gpui::px(0.),
                    gpui::TextAlign::Center => (bounds.size.width - width) / 2.,
                    gpui::TextAlign::Right => bounds.size.width - width,
                };
                let offset = base.x_for_index(start) - alignment;
                let left = origin.x + (base.x_for_index(a) - offset);
                let right = origin.x + (base.x_for_index(b) - offset);
                let rect = gpui::Bounds::from_corners(
                    gpui::point(left.min(right), y + height * row as f32),
                    gpui::point(left.max(right), y + height * (row + 1) as f32),
                );
                result.push(rect);
            }
            first += base.len + 1;
            y += height * starts.len() as f32;
        }
        result
    }
    pub(super) fn line_height(&self) -> gpui::Pixels {
        match self {
            Self::Plain(layout) => layout.line_height(),
            Self::Tabbed(layout) => layout.0.borrow().line_height,
        }
    }
}

#[derive(Clone)]
pub(super) struct TabLayout(Rc<RefCell<State>>);
struct State {
    lines: Vec<gpui::WrappedLine>,
    sources: Vec<String>,
    bases: Vec<Arc<gpui::LineLayout>>,
    line_height: gpui::Pixels,
    tab_width: f32,
    space_width: f32,
    bounds: gpui::Bounds<gpui::Pixels>,
    align: gpui::TextAlign,
    unwrapped_width: f32,
    precise: bool,
}

impl TabLayout {
    #[cfg(all(test, feature = "native-test"))]
    pub(super) fn new(
        text: gpui::SharedString,
        runs: Vec<gpui::TextRun>,
        tab_size: usize,
        window: &mut gpui::Window,
    ) -> Self {
        Self::with_line_height_snap(text, runs, tab_size, true, window)
    }

    pub(super) fn with_line_height_snap(
        text: gpui::SharedString,
        runs: Vec<gpui::TextRun>,
        tab_size: usize,
        snap: bool,
        window: &mut gpui::Window,
    ) -> Self {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = style
            .line_height
            .to_pixels(font_size.into(), window.rem_size());
        let line_height = if snap {
            window.pixel_snap(line_height)
        } else {
            line_height
        };
        // A tab and a space both occupy one UTF-8 byte. Shape spaces to obtain
        // invisible glyphs, then replace their advances without changing indices.
        let shaped = text.replace('\t', " ");
        let lines = window
            .text_system()
            .shape_text(shaped.into(), font_size, &runs, None, None)
            .map(|lines| lines.into_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        let space =
            window
                .text_system()
                .shape_line(" ".into(), font_size, &[style.to_run(1)], None);
        let space_width = f32::from(space.width).max(0.01);
        let bases = lines
            .iter()
            .map(|line| line.unwrapped_layout.clone())
            .collect();
        let mut state = State {
            lines,
            sources: text.split('\n').map(str::to_owned).collect(),
            bases,
            line_height,
            tab_width: space_width * tab_size.clamp(1, 32) as f32,
            space_width,
            bounds: Default::default(),
            align: style.text_align,
            unwrapped_width: 0.0,
            precise: !snap,
        };
        let size = state.layout(None, None);
        state.unwrapped_width = f32::from(size.width);
        Self(Rc::new(RefCell::new(state)))
    }
    pub(super) fn unwrapped_width(&self) -> f32 {
        self.0.borrow().unwrapped_width
    }
    pub(super) fn request_layout(&self, window: &mut gpui::Window) -> gpui::LayoutId {
        let style = window.text_style();
        let state = self.0.clone();
        let precise = state.borrow().precise;
        let measure = move |known: gpui::Size<Option<gpui::Pixels>>,
                            available: gpui::Size<gpui::AvailableSpace>,
                            __window: &mut gpui::Window,
                            _: &mut gpui::App| {
            let width = if style.white_space == gpui::WhiteSpace::Normal {
                known.width.or(match available.width {
                    gpui::AvailableSpace::Definite(width) => Some(width),
                    _ => None,
                })
            } else {
                None
            };
            state.borrow_mut().layout(width, style.line_clamp)
        };
        if precise {
            window.request_precise_measured_layout(Default::default(), measure)
        } else {
            window.request_measured_layout(Default::default(), measure)
        }
    }
    pub(super) fn prepaint(&self, bounds: gpui::Bounds<gpui::Pixels>) {
        self.0.borrow_mut().bounds = bounds;
    }
    pub(super) fn paint(&self, window: &mut gpui::Window, cx: &mut gpui::App) {
        let state = self.0.borrow();
        let mut origin = state.bounds.origin;
        for line in &state.lines {
            let _ = line.paint_background(
                origin,
                state.line_height,
                state.align,
                Some(state.bounds),
                window,
                cx,
            );
            let _ = line.paint(
                origin,
                state.line_height,
                state.align,
                Some(state.bounds),
                window,
                cx,
            );
            origin.y += line.size(state.line_height).height;
        }
    }
    fn position_for_index(&self, index: usize) -> Option<gpui::Point<gpui::Pixels>> {
        let state = self.0.borrow();
        let mut origin = state.bounds.origin;
        let mut first = 0;
        for line in &state.lines {
            if index >= first && index <= first + line.len() {
                return line
                    .position_for_index(index - first, state.line_height)
                    .map(|point| origin + point);
            }
            first += line.len() + 1;
            origin.y += line.size(state.line_height).height;
        }
        None
    }
    pub(super) fn closest_index_for_position(&self, point: gpui::Point<gpui::Pixels>) -> usize {
        let state = self.0.borrow();
        let mut origin = state.bounds.origin;
        let mut first = 0;
        for line in &state.lines {
            let bottom = origin.y + line.size(state.line_height).height;
            if point.y < bottom {
                return first + super::nearest_caret(line, point - origin, state.line_height);
            }
            first += line.len() + 1;
            origin.y = bottom;
        }
        first.saturating_sub(1)
    }

    fn index_for_position(&self, point: gpui::Point<gpui::Pixels>) -> Result<usize, usize> {
        let state = self.0.borrow();
        if point.y < state.bounds.top() {
            return Err(0);
        }
        let mut origin = state.bounds.origin;
        let mut first = 0;
        for line in &state.lines {
            let bottom = origin.y + line.size(state.line_height).height;
            if point.y <= bottom {
                return match line.index_for_position(point - origin, state.line_height) {
                    Ok(index) => Ok(first + index),
                    Err(index) => Err(first + index),
                };
            }
            first += line.len() + 1;
            origin.y = bottom;
        }
        Err(first.saturating_sub(1))
    }
}

impl State {
    fn layout(
        &mut self,
        wrap: Option<gpui::Pixels>,
        max_lines: Option<usize>,
    ) -> gpui::Size<gpui::Pixels> {
        let mut size: gpui::Size<gpui::Pixels> = gpui::Size::default();
        for ((line, base), source) in self.lines.iter_mut().zip(&self.bases).zip(&self.sources) {
            let (layout, boundaries) = tabbed_line(
                base,
                source,
                self.space_width,
                self.tab_width,
                wrap,
                max_lines,
            );
            *std::ops::DerefMut::deref_mut(line) = Arc::new(gpui::WrappedLineLayout {
                unwrapped_layout: Arc::new(layout),
                wrap_boundaries: boundaries.into_iter().collect(),
                wrap_width: wrap,
            });
            let measured = line.size(self.line_height);
            size.width = size.width.max(measured.width);
            size.height += measured.height;
        }
        size
    }
}

struct Cluster {
    first: usize,
    end: usize,
    x: f32,
    width: f32,
    tab: bool,
    whitespace: bool,
    opportunity: bool,
}

fn tabbed_line(
    base: &gpui::LineLayout,
    text: &str,
    space: f32,
    stop: f32,
    wrap: Option<gpui::Pixels>,
    max_lines: Option<usize>,
) -> (gpui::LineLayout, Vec<gpui::WrapBoundary>) {
    let opportunities = text
        .split_word_bound_indices()
        .map(|(index, _)| index)
        .collect::<std::collections::HashSet<_>>();
    let clusters = text
        .grapheme_indices(true)
        .map(|(first, value)| {
            let end = first + value.len();
            let x = f32::from(base.x_for_index(first));
            Cluster {
                first,
                end,
                x,
                width: f32::from(base.x_for_index(end)) - x,
                tab: value == "\t",
                whitespace: value.chars().all(char::is_whitespace),
                opportunity: opportunities.contains(&first)
                    && !value.chars().all(char::is_whitespace),
            }
        })
        .collect::<Vec<_>>();
    let advance = |cluster: &Cluster, x: f32| {
        if !cluster.tab {
            return cluster.width;
        }
        let mut next = ((x / stop).floor() + 1.0) * stop;
        if next - x < space * 0.5 - 0.001 {
            next += stop;
        }
        next - x
    };
    let mut positions = vec![0.0; clusters.len()];
    let mut boundaries = Vec::new();
    let mut start = 0;
    let mut absolute = 0.0;
    while start < clusters.len() {
        let mut x = 0.0;
        let mut candidate = None;
        let mut end = clusters.len();
        let can_wrap = max_lines.is_none_or(|maximum| boundaries.len() + 1 < maximum);
        for (index, cluster) in clusters.iter().enumerate().skip(start) {
            if index > start && cluster.opportunity {
                candidate = Some(index);
            }
            let width = advance(cluster, x);
            if can_wrap
                && !cluster.whitespace
                && wrap.is_some_and(|limit| x + width > f32::from(limit))
                && index > start
            {
                end = candidate.unwrap_or(index);
                break;
            }
            x += width;
        }
        x = 0.0;
        for (index, cluster) in clusters.iter().enumerate().take(end).skip(start) {
            positions[index] = absolute + x;
            x += advance(cluster, x);
        }
        absolute += x;
        if end < clusters.len() {
            let byte = clusters[end].first;
            if let Some(boundary) = base.runs.iter().enumerate().find_map(|(run_ix, run)| {
                run.glyphs
                    .iter()
                    .position(|glyph| glyph.index >= byte)
                    .map(|glyph_ix| gpui::WrapBoundary { run_ix, glyph_ix })
            }) {
                boundaries.push(boundary);
            }
        }
        start = end;
    }
    let mut runs = base.runs.clone();
    for run in &mut runs {
        for glyph in &mut run.glyphs {
            let index = clusters.partition_point(|cluster| cluster.end <= glyph.index);
            if let Some(cluster) = clusters.get(index) {
                glyph.position.x += gpui::px(positions[index] - cluster.x);
            }
        }
    }
    (
        gpui::LineLayout {
            font_size: base.font_size,
            width: gpui::px(absolute),
            ascent: base.ascent,
            descent: base.descent,
            runs,
            len: base.len,
        },
        boundaries,
    )
}

#[cfg(all(test, feature = "native-test"))]
mod tests {
    use super::*;

    #[gpui::test]
    fn link_rectangle_unions_wrapped_rows_after_reflow(cx: &mut gpui::TestAppContext) {
        let cx = cx.add_empty_window();
        let text = "Hello @long-user-name!";
        let layout = cx.update(|window, _cx| {
            TabLayout::new(
                text.into(),
                vec![window.text_style().to_run(text.len())],
                4,
                window,
            )
        });
        layout.prepaint(gpui::Bounds::new(
            gpui::point(gpui::px(20.), gpui::px(40.)),
            gpui::size(gpui::px(400.), gpui::px(100.)),
        ));
        let rich = RichLayout::Tabbed(layout.clone());
        let wide = rich.bounds_for_range(6..21).unwrap();
        layout.0.borrow_mut().layout(Some(gpui::px(70.)), None);
        let wrapped = rich.bounds_for_range(6..21).unwrap();
        assert!(
            wrapped.size.height > wide.size.height,
            "{wide:?} -> {wrapped:?}"
        );
        assert!(
            wrapped.size.width < wide.size.width,
            "{wide:?} -> {wrapped:?}"
        );
        assert!(wrapped.origin.x >= gpui::px(20.));
        assert!(wrapped.origin.y >= gpui::px(40.));
        assert!(rich.bounds_for_range(0..0).is_none());
    }
}
