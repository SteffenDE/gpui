defmodule GPUI.Test.Native.HorizontalTextTest do
  use GPUI.Test, native: [size: {360, 240}]

  defmodule View do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col w-[300px]">
        <div id="old-row" class="flex h-5 w-full">
          <div style={[width: {:px, Map.get(assigns, :gutter, 0) * 1.0}]} class="shrink-0" />
          <UI.rich_text id="old-code" label="Old code" runs={[]} text={String.duplicate("x", Map.get(assigns, :old_length, 400))}
            scroll_group="file" scroll_pane="old" scroll_revision={assigns.revision} phx-scroll="old_scroll"
            reveal_start={Map.get(assigns, :reveal_start)} reveal_end={Map.get(assigns, :reveal_end)} reveal_request={Map.get(assigns, :reveal_request, 0)}
            class="flex-1 min-w-0 h-5 text-[13px] whitespace-nowrap" />
        </div>
        <div id="new-row" class="flex h-5 w-full">
          <div style={[width: {:px, Map.get(assigns, :gutter, 0) * 1.0}]} class="shrink-0" />
          <UI.rich_text id="new-code" label="New code" runs={[]} text={String.duplicate("y", Map.get(assigns, :new_length, 100))}
            scroll_group="file" scroll_pane="new" scroll_revision={assigns.revision} phx-scroll="new_scroll"
            class="flex-1 min-w-0 h-5 text-[13px] whitespace-nowrap" />
        </div>
        {if Map.get(assigns, :scrollbar, false) do
          UI.rich_text(%{id: "common-bar", label: "Horizontal scrollbar", text: "", runs: [], selectable: false,
            scroll_group: "file", scroll_pane: "old", scroll_revision: assigns.revision,
            scrollbar: true, scrollbar_gutter: Map.get(assigns, :gutter, 0),
            scrollbar_visible: Map.get(assigns, :visible, true),
            "phx-scroll": "old_scroll", class: "w-full h-[6px]"})
        else
          []
        end}
      </div>
      """
    end
  end

  test "horizontal text panes share offsets while retaining their own limits", %{ui: ui} do
    render(ui, View, revision: 1)
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "old_scroll", value: [old_left, old_max, _, _]}}}

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "new_scroll", value: [new_left, new_max, _, _]}}}

    assert old_left == 0
    assert new_left == 0
    assert old_max > new_max
    assert new_max > 120
    scroll(ui, "old-row", delta: {-120, 0})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "old_scroll", value: [120.0, _, _, _]}}}
    assert_receive {:gpui, ^ui, {:event, %{event: "new_scroll", value: [120.0, _, _, _]}}}
    click(ui, "old-row")
    press(ui, :arrow_right)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "old_scroll", value: [160.0, _, _, _]}}}
    assert_receive {:gpui, ^ui, {:event, %{event: "new_scroll", value: [160.0, _, _, _]}}}
    render(ui, View, revision: 1)
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "old_scroll"}}}, 20
    scroll(ui, "old-row", delta: {-9_000, 0})
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "old_scroll", value: [left, maximum, _, _]}}}
                   when left == maximum

    assert_receive {:gpui, ^ui, {:event, %{event: "new_scroll", value: [left, maximum, _, _]}}}
                   when left == maximum

    render(ui, View, revision: 2, old_length: 10, new_length: 5)
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "old_scroll", value: [left, maximum, _, _]}}}
                   when left == 0 and maximum == 0

    assert_receive {:gpui, ^ui, {:event, %{event: "new_scroll", value: [left, maximum, _, _]}}}
                   when left == 0 and maximum == 0
  end

  test "a full-width common scrollbar retains the narrower code viewport and synchronized drag",
       %{ui: ui} do
    attrs = [revision: 1, gutter: 40, scrollbar: true, visible: true]
    render(ui, View, attrs)
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "old_scroll", value: [_, maximum, width, extent]}}}
                   when width == 260 and maximum > 0

    track_width = 300.0
    length = max(17.0, track_width * track_width / (extent + 40))
    drag(ui, {length / 2, 35}, {length / 2, 43})
    drag(ui, {length / 2, 43}, {length / 2 + 60, 43})
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "old_scroll", value: [left, _, width, _]}}}
                   when left > 100 and width == 260

    assert_receive {:gpui, ^ui, {:event, %{event: "new_scroll", value: [left, _, width, _]}}}
                   when left > 100 and width == 260

    render(ui, View, attrs)
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "old_scroll"}}}, 20
    render(ui, View, Keyword.put(attrs, :visible, false))
    settle(ui)
    # Hidden thumbs do not accept page clicks, and a separate bar does not
    # overwrite the shaped code row's bounds or horizontal scroll metrics.
    drag(ui, {290, 35}, {290, 43})
    click(ui, {290, 43})
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "old_scroll"}}}, 20
    expected = 353 * (extent - 8) / 400 - 130
    render(ui, View, attrs ++ [reveal_start: 350, reveal_end: 356, reveal_request: 1])
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "old_scroll", value: [left, _, width, _]}}}
                   when abs(left - expected) < 1 and width == 260
  end
end
