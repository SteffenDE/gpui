defmodule GPUI.Test.Native.TooltipTrackingTest do
  use GPUI.Test, native: [size: {320, 160}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div style={[preserve_layout_bounds: true]} class="flex flex-col items-start">
        <div id="first" class="flex">{hint("⌘⇧F", 0.5)}</div>
        <div id="second" class="flex">{hint("⌘⇧F", assigns.spacing)}</div>
        <div id="combining-first" class="flex">{hint("e\u0301F", 0.5)}</div>
        <div id="combining-second" class="flex">{hint("e\u0301F", assigns.spacing)}</div>
      </div>
      """
    end

    defp hint(text, spacing) do
      %GPUI.Element{
        type: :ui_tooltip_shortcut,
        attrs: [text: text, letter_spacing: spacing],
        children: []
      }
    end
  end

  test "shortcut tracking contributes one fractional advance per grapheme and updates on rerender",
       %{ui: ui} do
    render(ui, View, spacing: 1.7)
    settle(ui)
    assert_in_delta bounds(ui, "second").width - bounds(ui, "first").width, 3.6, 0.001

    assert_in_delta bounds(ui, "combining-second").width - bounds(ui, "combining-first").width,
                    2.4,
                    0.001

    render(ui, View, spacing: 0.5)
    settle(ui)
    assert bounds(ui, "second").width == bounds(ui, "first").width
    assert bounds(ui, "combining-second").width == bounds(ui, "combining-first").width
  end
end
