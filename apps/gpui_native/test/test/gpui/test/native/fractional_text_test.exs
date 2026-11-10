defmodule GPUI.Test.Native.FractionalTextTest do
  use GPUI.Test, native: [size: {360, 600}]
  alias GPUI.Text.{Position, Range}

  defmodule View do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      ~GPUI"""
      <UI.rich_text id="prose" label="Prose" text={assigns.text} runs={[]}
        line_height_snap={Map.get(assigns, :snap, "none")}
        geometry_ranges={assigns.ranges} phx-range-geometry-change={assigns.event}
        phx-change="selection" hit_test_x={Map.get(assigns, :hit_x)} hit_test_y={Map.get(assigns, :hit_y)}
        hit_test_request={Map.get(assigns, :hit_request, 0)} phx-hit-test="caret"
        style={[width: {:px, Map.get(assigns, :width, 300) * 1.0}]}
        class={"text-[15px] leading-[22.3px] " <> if(Map.get(assigns, :truncate, false), do: "truncate", else: "whitespace-normal")} />
      """
    end
  end

  test "physical lines retain fractional advances and device mode remains available", %{ui: ui} do
    ranges = [span(0, 0, 0, 3), span(1, 0, 1, 3)]
    attrs = [text: "one\ntwo", ranges: ranges, event: "fractional"]
    render(ui, View, attrs)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "fractional", value: [a, b]}}}
    [first] = a.rectangles
    [second] = b.rectangles
    assert_in_delta first.height, 22.3, 0.001
    assert_in_delta second.y - first.y, 22.3, 0.001
    render(ui, View, Keyword.merge(attrs, snap: "device", event: "device"))
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "device", value: [a, b]}}}
    [first] = a.rectangles
    [second] = b.rectangles
    refute_in_delta first.height, 22.3, 0.01
    assert_in_delta second.y - first.y, first.height, 0.001
  end

  test "native wrapping retains widths and fractional visual-row geometry", %{ui: ui} do
    text = "one two three four five"
    attrs = [text: text, ranges: [span(0, 0, 0, String.length(text))], width: 70]
    render(ui, View, Keyword.merge(attrs, snap: "device", event: "device-wrap"))
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "device-wrap", value: [%{rectangles: snapped}]}}}

    render(ui, View, Keyword.put(attrs, :event, "fractional-wrap"))
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "fractional-wrap", value: [%{rectangles: fractional}]}}}

    assert length(fractional) > 1
    assert length(fractional) == length(snapped)

    for {rect, original} <- Enum.zip(fractional, snapped) do
      assert_in_delta rect.width, original.width, 0.001
      assert_in_delta rect.height, 22.3, 0.001
    end

    for [first, second] <- Enum.chunk_every(fractional, 2, 1, :discard),
        do: assert_in_delta(second.y - first.y, 22.3, 0.001)
  end

  test "tabs, Unicode caret queries, and selection use fractional rows", %{ui: ui} do
    text = "🌱\tZ\ncafé"
    render(ui, View, text: text, ranges: [span(1, 0, 1, 4)], event: "tabs")
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "tabs", value: [%{rectangles: [rect]}]}}}
    assert_in_delta rect.y, 22.3, 0.001

    render(ui, View,
      text: text,
      ranges: [span(1, 0, 1, 4)],
      event: "tabs",
      hit_x: rect.x + 0.1,
      hit_y: rect.y + 2,
      hit_request: 1
    )

    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "caret", value: [line, column]}}}
    assert {line, column} == {1, 0}
    click(ui, "prose")
    press(ui, "cmd-a")
    assert_receive {:gpui, ^ui, {:event, %{event: "selection", value: ^text}}}
  end

  test "fractional text retains native ellipsis truncation", %{ui: ui} do
    for snap <- ["device", "none"] do
      event = "truncate-" <> snap

      render(ui, View,
        text: "one two three four five",
        ranges: [span(0, 0, 0, 3), span(0, 20, 0, 22)],
        width: 70,
        truncate: true,
        snap: snap,
        event: event
      )

      settle(ui)
      assert_receive {:gpui, ^ui, {:event, %{event: ^event, value: [first, last]}}}
      assert [%{width: width}] = first.rectangles
      assert width > 0
      assert last.rectangles == []
    end
  end

  defmodule StackView do
    use GPUI.View
    alias GPUI.UI

    def render(_assigns) do
      ~GPUI"""
      <div class="flex flex-col w-full">
        {for index <- 0..19 do
          UI.rich_text(%{id: "row-#{index}", label: "Row", text: "row", runs: [],
            line_height_snap: "none", class: "w-full text-[15px] leading-[22.3px]"})
        end}
      </div>
      """
    end
  end

  test "fractional advances accumulate across separately laid-out paragraphs", %{ui: ui} do
    render(ui, StackView, %{})
    settle(ui)
    first = bounds(ui, "row-0")
    last = bounds(ui, "row-19")
    assert_in_delta last.y - first.y, 19 * 22.3, 0.51
  end

  defp span(a, b, c, d), do: Range.new(Position.new(a, b), Position.new(c, d))
end
