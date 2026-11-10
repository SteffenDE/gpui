defmodule GPUI.Test.Native.RichGeometryTest do
  use GPUI.Test, native: [size: {360, 240}]
  alias GPUI.Text.{Position, Range}

  defmodule View do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col w-full" style={[padding_top: {:px, Map.get(assigns, :top, 0) * 1.0}]}>
        <UI.rich_text id="document" label="Document" text={assigns.text} runs={[]}
          geometry_ranges={assigns.ranges} phx-range-geometry-change={Map.get(assigns, :event, "ranges")}
          phx-change="selected"
          style={[width: {:px, Map.get(assigns, :width, 300) * 1.0}]}
          class={"text-[13px] leading-[20px] whitespace-normal " <> Map.get(assigns, :alignment, "text-left")} />
      </div>
      """
    end
  end

  test "range queries preserve UTF-16 coordinates across emoji and physical lines", %{ui: ui} do
    ranges = [text_range(0, 2, 0, 3), text_range(1, 0, 1, 4), text_range(0, 0, 0, 2)]
    render(ui, View, text: "🌱Z\ncafé", ranges: ranges)
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{type: :range_geometry, value: geometries, revision: 0}}}

    assert Enum.map(geometries, & &1.range) ==
             Enum.map(ranges, fn r ->
               %{start: Map.from_struct(r.start), end: Map.from_struct(r.end)}
             end)

    [z, cafe, emoji] = Enum.map(geometries, &hd(&1.rectangles))
    assert z.x > emoji.x
    assert_in_delta z.y, emoji.y, 0.1
    assert_in_delta cafe.y - z.y, 20, 0.1
    assert cafe.width > z.width
    refute_receive {:gpui, ^ui, {:event, %{event: "selected"}}}
  end

  test "soft-wrapped matches expose each visual row and update after layout changes", %{ui: ui} do
    attrs = [text: "abcdZ", ranges: [text_range(0, 1, 0, 5)], width: 30]
    render(ui, View, attrs)
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "ranges", value: [%{rectangles: [first, last]}]}}}

    assert first.x > last.x
    assert_in_delta last.y - first.y, 20, 0.1
    render(ui, View, attrs)
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "ranges"}}}
    render(ui, View, Keyword.merge(attrs, width: 300, top: 40))
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "ranges", value: [%{rectangles: [wide]}]}}}
    assert_in_delta wide.y - first.y, 40, 0.1
    assert wide.width > first.width
  end

  test "tabs retain source ranges and changed callbacks receive the current geometry", %{ui: ui} do
    attrs = [text: "a\tZ", ranges: [text_range(0, 2, 0, 3)]]
    render(ui, View, attrs)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "ranges", value: [%{rectangles: [tabbed]}]}}}
    assert tabbed.x > 10
    render(ui, View, Keyword.put(attrs, :event, "new-visit"))
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "new-visit", value: [%{rectangles: [same]}]}}}
    assert tabbed == same
    render(ui, View, text: "a\tZ", ranges: [])
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "ranges", value: []}}}
  end

  test "empty and out-of-document ranges return no rectangles", %{ui: ui} do
    render(ui, View, text: "🌱Z", ranges: [text_range(0, 2, 0, 2), text_range(9, 0, 9, 1)])
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{value: [%{rectangles: []}, %{rectangles: []}]}}}
  end

  defmodule ScrollView do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      ~GPUI"""
      <UI.virtual_collection id="viewport" label="Preview" measure_all={true} class="w-[300px] h-[120px]">
        <UI.virtual_item id="document">
        <div class="flex flex-col w-full">
          <UI.rich_text id="long-document" label="Long document" text={assigns.text} runs={[]}
            geometry_ranges={assigns.ranges} phx-range-geometry-change="scrolled-ranges"
            class="w-full text-[13px] leading-[20px] whitespace-normal" />
        </div>
        </UI.virtual_item>
      </UI.virtual_collection>
      """
    end
  end

  test "range coordinates track scrolling even when the match starts outside the viewport", %{
    ui: ui
  } do
    render(ui, ScrollView,
      text: String.duplicate("Orchard\n", 40),
      ranges: [text_range(25, 0, 25, 7)]
    )

    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "scrolled-ranges", value: [%{rectangles: [before]}]}}}

    assert before.y >= 500
    scroll(ui, "viewport", delta: {0, -300})
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "scrolled-ranges", value: [%{rectangles: [after_scroll]}]}}}

    assert_in_delta before.y - after_scroll.y, 300, 0.1
    assert_in_delta before.x, after_scroll.x, 0.1
  end

  test "range rectangles follow centered and right-aligned shaping", %{ui: ui} do
    attrs = [text: "abc", ranges: [text_range(0, 0, 0, 3)], width: 300]

    for {alignment, fraction} <- [{"text-center", 0.5}, {"text-right", 1.0}] do
      render(ui, View, Keyword.put(attrs, :alignment, alignment))
      settle(ui)
      assert_receive {:gpui, ^ui, {:event, %{value: [%{rectangles: [rectangle]}]}}}
      assert_in_delta rectangle.x, (300 - rectangle.width) * fraction, 0.1
    end
  end

  defp text_range(a, b, c, d), do: Range.new(Position.new(a, b), Position.new(c, d))
end
