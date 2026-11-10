use crate::gpui::{
    actions, div, fill, point, px, rgba, size, App, Bounds, ClipboardItem, Context, CursorStyle,
    Element, ElementId, ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, KeyBinding, LayoutId,
    MouseButton, MouseDownEvent, PaintQuad, ParentElement, Pixels, Render, ShapedLine, Style,
    Styled, TextAlign, TextRun, UTF16Selection, UnderlineStyle, Window,
};
use crate::{gpui, push_event, EventValue, InputKind, NativeEvent, SharedRuntime};
use std::{
    collections::{HashSet, VecDeque},
    ops::Range,
};
use unicode_segmentation::UnicodeSegmentation;

#[cfg(feature = "real-gpui")]
actions!(
    gpui_elixir_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        SelectHome,
        SelectEnd,
        Home,
        End,
        Paste,
        Copy,
        Cut
    ]
);

pub(crate) fn bind_input_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-left", Home, None),
        KeyBinding::new("cmd-right", End, None),
        KeyBinding::new("cmd-shift-left", SelectHome, None),
        KeyBinding::new("cmd-shift-right", SelectEnd, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
    ]);
}

fn set_input_value(runtime: &SharedRuntime, input_id: &str, value: String) {
    if let Ok(mut values) = runtime.input_values.lock() {
        values.insert(input_id.to_string(), value);
    }
}

pub(crate) struct NativeTextInput {
    id: String,
    runtime: SharedRuntime,
    window_id: u64,
    change: Option<String>,
    placeholder: String,
    focus_handle: FocusHandle,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    confirmed_value: String,
    pending_values: VecDeque<String>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    last_text_origin: Option<gpui::Point<Pixels>>,
    select_all_request: u64,
    capture_keys: Vec<String>,
    capture_keydown: Option<String>,
    capture_keystroke: Option<String>,
    capture_pressed: HashSet<String>,
    _capture_subscription: gpui::Subscription,
}

impl NativeTextInput {
    pub(crate) fn new(
        id: String,
        runtime: SharedRuntime,
        window_id: u64,
        value: String,
        placeholder: Option<String>,
        change: Option<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        set_input_value(&runtime, &id, value.clone());
        let input = cx.weak_entity();
        let capture_subscription = cx.intercept_keystrokes(move |event, window, cx| {
            let mods = event.keystroke.modifiers;
            if mods.control || mods.alt || mods.platform || mods.function {
                return;
            }
            let capture_key = if mods.shift {
                if event.keystroke.key != "tab" {
                    return;
                }
                "shift-tab"
            } else {
                event.keystroke.key.as_str()
            };
            let Some(input) = input.upgrade() else {
                return;
            };
            input.update(cx, |input, cx| {
                if !input.focus_handle.is_focused(window)
                    || !input.capture_keys.iter().any(|key| key == capture_key)
                {
                    return;
                }
                if event.keystroke.key == "escape"
                    && input.runtime.overlay_owns_escape(input.window_id)
                {
                    return;
                }
                let repeat = !input.capture_pressed.insert(event.keystroke.key.clone());
                if let Some(callback) = input.capture_keystroke.as_ref() {
                    crate::element::event::emit_structured_key(
                        &input.runtime,
                        input.window_id,
                        callback,
                        &event.keystroke,
                        repeat,
                    );
                }
                if let Some(callback) = input.capture_keydown.as_ref() {
                    let _ = push_event(
                        &input.runtime,
                        NativeEvent::Input {
                            kind: InputKind::KeyDown,
                            window_id: input.window_id,
                            event: callback.clone(),
                            value: Some(EventValue::String(event.keystroke.key.clone())),
                        },
                    );
                }
                cx.stop_propagation();
            });
        });
        Self {
            id,
            runtime,
            window_id,
            change,
            placeholder: placeholder.unwrap_or_default(),
            focus_handle: cx.focus_handle(),
            selected_range: value.len()..value.len(),
            selection_reversed: false,
            marked_range: None,
            confirmed_value: value,
            pending_values: VecDeque::new(),
            last_layout: None,
            last_bounds: None,
            last_text_origin: None,
            select_all_request: 0,
            capture_keys: Vec::new(),
            capture_keydown: None,
            capture_keystroke: None,
            capture_pressed: HashSet::new(),
            _capture_subscription: capture_subscription,
        }
    }

    pub(crate) fn request_select_all(&mut self, request: u64) {
        if request > 0 && request != self.select_all_request {
            self.selected_range = 0..self.value().len();
            self.selection_reversed = false;
            self.marked_range = None;
        }
        self.select_all_request = request;
    }

    pub(crate) fn set_key_capture(
        &mut self,
        keys: Vec<String>,
        keydown: Option<String>,
        keystroke: Option<String>,
    ) {
        self.capture_keys = keys;
        self.capture_keydown = keydown;
        self.capture_keystroke = keystroke;
    }

    pub(crate) fn native_focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    pub(crate) fn update_props(
        &mut self,
        value: String,
        placeholder: Option<String>,
        change: Option<String>,
    ) {
        let apply_value = self.reconcile_value(&value);

        let previous = self.value();
        if apply_value && previous != value {
            let at_end = self.selected_range.is_empty() && self.cursor_offset() == previous.len();
            let cursor = if at_end {
                value.len()
            } else {
                Self::clamp(&value, self.cursor_offset())
            };
            set_input_value(&self.runtime, &self.id, value.clone());
            self.selected_range = cursor..cursor;
            self.selection_reversed = false;
            self.marked_range = None;
        }

        self.placeholder = placeholder.unwrap_or_default();
        self.change = change;
    }

    fn value(&self) -> String {
        self.runtime
            .input_values
            .lock()
            .ok()
            .and_then(|values| values.get(&self.id).cloned())
            .unwrap_or_default()
    }

    // Key events can rerender the view before their associated change event is handled.
    // Do not let those stale snapshots overwrite newer local edits.
    fn reconcile_value(&mut self, value: &str) -> bool {
        if let Some(index) = self
            .pending_values
            .iter()
            .position(|pending| pending == value)
        {
            self.pending_values.drain(..=index);
            self.confirmed_value = value.to_string();
            return false;
        }

        if !self.pending_values.is_empty() && value == self.confirmed_value {
            return false;
        }

        self.pending_values.clear();
        self.confirmed_value = value.to_string();
        true
    }

    fn emit_change(&mut self, value: String) {
        if let Some(event) = self.change.as_ref() {
            self.pending_values.push_back(value.clone());

            if push_event(
                &self.runtime,
                NativeEvent::Input {
                    kind: InputKind::Change,
                    window_id: self.window_id,
                    event: event.clone(),
                    value: Some(EventValue::String(value)),
                },
            )
            .is_err()
            {
                self.pending_values.pop_back();
            }
        }
    }

    fn clamp(value: &str, mut offset: usize) -> usize {
        offset = offset.min(value.len());
        while offset > 0 && !value.is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        Self::offset_from_utf16_for(&self.value(), offset)
    }

    fn offset_from_utf16_for(value: &str, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in value.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn offset_to_utf16_for(value: &str, offset: usize) -> usize {
        let offset = Self::clamp(value, offset);
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in value.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range.start)..self.offset_from_utf16(range.end)
    }

    fn marked_selection_range(
        new_text: &str,
        inserted: &Range<usize>,
        selection: &Range<usize>,
    ) -> Range<usize> {
        let start = Self::offset_from_utf16_for(new_text, selection.start);
        let end = Self::offset_from_utf16_for(new_text, selection.end.max(selection.start));
        let start = inserted.start.saturating_add(start).min(inserted.end);
        let end = inserted.start.saturating_add(end).min(inserted.end);
        start..end
    }

    fn range_to_utf16_for(value: &str, range: &Range<usize>) -> Range<usize> {
        Self::offset_to_utf16_for(value, range.start)..Self::offset_to_utf16_for(value, range.end)
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn replace_value(&mut self, range: Range<usize>, text: &str) -> Option<Range<usize>> {
        let updated = self.runtime.input_values.lock().ok().map(|mut values| {
            let value = values.entry(self.id.clone()).or_default();
            let start = Self::clamp(value, range.start);
            let end = Self::clamp(value, range.end.max(start));
            value.replace_range(start..end, text);
            let inserted_end = start.saturating_add(text.len()).min(value.len());
            (value.clone(), start..inserted_end)
        });

        updated.map(|(value, inserted)| {
            self.selected_range = inserted.end..inserted.end;
            self.emit_change(value);
            inserted
        })
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        let value = self.value();
        value
            .grapheme_indices(true)
            .map(|(idx, _)| idx)
            .rfind(|idx| *idx < offset)
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        let value = self.value();
        value
            .grapheme_indices(true)
            .map(|(idx, _)| idx)
            .find(|idx| *idx > offset)
            .unwrap_or(value.len())
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let value = self.value();
        let offset = Self::clamp(&value, offset);
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let value = self.value();
        let offset = Self::clamp(&value, offset);
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn point_index(&self, position: gpui::Point<Pixels>) -> usize {
        let value = self.value();
        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return value.len();
        };
        if position.y < bounds.top() {
            0
        } else if position.y > bounds.bottom() {
            value.len()
        } else {
            line.closest_index_for_x(position.x - self.last_text_origin.unwrap_or(bounds.origin).x)
        }
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.value().len();
        self.selection_reversed = false;
        cx.notify();
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.value().len(), cx);
    }

    fn select_home(&mut self, _: &SelectHome, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(0, cx);
    }

    fn select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.value().len(), cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if prev == self.cursor_offset() {
                window.play_system_bell();
                return;
            }
            self.select_to(prev, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if next == self.cursor_offset() {
                window.play_system_bell();
                return;
            }
            self.select_to(next, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace('\n', " "), window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            let value = self.value();
            cx.write_to_clipboard(ClipboardItem::new_string(
                value[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        self.copy(&Copy, window, cx);
        if !self.selected_range.is_empty() {
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if event.modifiers.shift {
            self.select_to(self.point_index(event.position), cx);
        } else {
            self.move_to(self.point_index(event.position), cx);
        }
    }
}

impl Focusable for NativeTextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for NativeTextInput {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let value = self.value();
        let range = self.range_from_utf16(&range);
        let start = Self::clamp(&value, range.start);
        let end = Self::clamp(&value, range.end.max(start));
        adjusted_range.replace(Self::range_to_utf16_for(&value, &(start..end)));
        Some(value[start..end].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let value = self.value();
        Some(UTF16Selection {
            range: Self::range_to_utf16_for(&value, &self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let value = self.value();
        self.marked_range
            .as_ref()
            .map(|range| Self::range_to_utf16_for(&value, range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());
        let _ = self.replace_value(range, text);
        self.marked_range = None;
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());
        if let Some(inserted) = self.replace_value(range, new_text) {
            self.marked_range = (!new_text.is_empty()).then_some(inserted.clone());
            if let Some(selection) = new_selected_range {
                self.selected_range = Self::marked_selection_range(new_text, &inserted, &selection);
            }
        }
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let line = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range);
        let origin = self.last_text_origin.unwrap_or(bounds.origin);
        Some(Bounds::from_corners(
            point(origin.x + line.x_for_index(range.start), origin.y),
            point(
                origin.x + line.x_for_index(range.end),
                origin.y + window.line_height(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let value = self.value();
        Some(Self::offset_to_utf16_for(&value, self.point_index(point)))
    }
}

struct NativeTextInputElement {
    input: Entity<NativeTextInput>,
}

struct NativeTextInputPrepaintState {
    origin: gpui::Point<Pixels>,
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for NativeTextInputElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeTextInputElement {
    type RequestLayoutState = ();
    type PrepaintState = NativeTextInputPrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = gpui::relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.value();
        let display_text = if content.is_empty() {
            input.placeholder.clone()
        } else {
            content.clone()
        };
        let style = window.text_style();
        // A primitive input inherits its foreground for both text and placeholder.
        // Consumers can select a muted foreground while the value is empty; a
        // hard-coded translucent black makes placeholders disappear in dark UIs.
        let color = style.color;
        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if content.is_empty() {
            vec![run]
        } else if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end.saturating_sub(marked_range.start),
                    underline: Some(UnderlineStyle {
                        color: Some(color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len().saturating_sub(marked_range.end),
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };
        let line = window.text_system().shape_line(
            display_text.into(),
            style.font_size.to_pixels(window.rem_size()),
            &runs,
            None,
        );
        let origin = text_origin(bounds, line.width, window.line_height(), style.text_align);
        let text_bounds = Bounds::new(origin, size(bounds.size.width, window.line_height()));
        let cursor_pos = line.x_for_index(input.cursor_offset());
        let (selection, cursor) = if input.selected_range.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(text_bounds.left() + cursor_pos, text_bounds.top()),
                        size(px(2.), text_bounds.bottom() - text_bounds.top()),
                    ),
                    gpui::blue(),
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            text_bounds.left() + line.x_for_index(input.selected_range.start),
                            text_bounds.top(),
                        ),
                        point(
                            text_bounds.left() + line.x_for_index(input.selected_range.end),
                            text_bounds.bottom(),
                        ),
                    ),
                    rgba(0x3311ff30),
                )),
                None,
            )
        };
        NativeTextInputPrepaintState {
            origin,
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }
        let Some(line) = prepaint.line.take() else {
            return;
        };
        if line
            .paint(
                prepaint.origin,
                window.line_height(),
                TextAlign::Left,
                None,
                window,
                cx,
            )
            .is_err()
        {
            return;
        }
        if focus_handle.is_focused(window) {
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        }
        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
            input.last_text_origin = Some(prepaint.origin);
        });
    }
}

fn text_origin(
    bounds: Bounds<Pixels>,
    line_width: Pixels,
    line_height: Pixels,
    align: TextAlign,
) -> gpui::Point<Pixels> {
    let remaining = (bounds.size.width - line_width).max(px(0.0));
    let x = match align {
        TextAlign::Center => remaining / 2.0,
        TextAlign::Right => remaining,
        _ => px(0.0),
    };
    point(
        bounds.left() + x,
        bounds.top() + (bounds.size.height - line_height).max(px(0.0)) / 2.0,
    )
}

#[cfg(test)]
mod tests {
    use super::NativeTextInput;

    #[test]
    fn aligned_text_origin_is_shared_by_paint_caret_hit_testing_and_ime() {
        use crate::gpui::{point, px, size, Bounds, TextAlign};
        let bounds = Bounds::new(point(px(10.0), px(20.0)), size(px(80.0), px(32.0)));
        for (align, x) in [
            (TextAlign::Left, 10.0),
            (TextAlign::Center, 40.0),
            (TextAlign::Right, 70.0),
        ] {
            assert_eq!(
                super::text_origin(bounds, px(20.0), px(20.0), align),
                point(px(x), px(26.0))
            );
        }
        assert_eq!(
            super::text_origin(bounds, px(100.0), px(40.0), TextAlign::Center),
            point(px(10.0), px(20.0))
        );
    }

    #[test]
    fn marked_selection_clamps_malformed_utf16_ranges_to_inserted_text() {
        let inserted = 4..9;

        assert_eq!(
            NativeTextInput::marked_selection_range("a😀", &inserted, &(1..3)),
            5..9
        );
        assert_eq!(
            NativeTextInput::marked_selection_range("a😀", &inserted, &(usize::MAX..usize::MAX),),
            9..9
        );
    }
}

impl Render for NativeTextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("GPUIInput")
            .track_focus(&self.focus_handle(cx))
            .on_key_up(cx.listener(|input, key: &gpui::KeyUpEvent, _window, _cx| {
                input.capture_pressed.remove(&key.keystroke.key);
            }))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_down_out(cx.listener(|input, _event, window, cx| {
                if input.focus_handle.is_focused(window) {
                    window.blur(cx);
                }
            }))
            .child(NativeTextInputElement { input: cx.entity() })
    }
}
