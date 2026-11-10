defmodule GPUI.Test.Native.TabsTest do
  use GPUI.Test, native: [size: {360, 240}]

  defmodule View do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      ~GPUI"""
      <UI.rich_text id="code" label="Tabbed code" text={assigns.text} runs={Map.get(assigns, :runs, [])}
        tab_size={Map.get(assigns, :tab_size, 2)} selectable={Map.get(assigns, :selectable, true)}
        phx-change="selected" phx-link="activated"
        hit_test_x={Map.get(assigns, :hit_x)} hit_test_y={Map.get(assigns, :hit_y)}
        hit_test_request={Map.get(assigns, :hit_request, 0)} phx-hit-test="caret"
        scroll_group={if Map.get(assigns, :horizontal, false), do: "tabs"} phx-scroll="scrolled"
        reveal_start={Map.get(assigns, :reveal_start)} reveal_end={Map.get(assigns, :reveal_end)} reveal_request={Map.get(assigns, :request, 0)}
        style={[width: {:px, Map.get(assigns, :width, 300) * 1.0}]}
        class={"text-[13px] leading-[20px] " <> if(Map.get(assigns, :wrap, false), do: "whitespace-normal", else: "whitespace-nowrap")} />
      """
    end
  end

  test "caret queries use shaped tabs and UTF-16 positions without selecting text", %{ui: ui} do
    render(ui, View, text: "a\tZ", hit_x: 18, hit_y: 8, hit_request: 1)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "caret", value: [line, column]}}}
    assert {line, column} == {0, 2}
    refute_receive {:gpui, ^ui, {:event, %{event: "selected"}}}
    render(ui, View, text: "a\tZ", hit_x: 18, hit_y: 8, hit_request: 1)
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "caret"}}}
    render(ui, View, text: "🌱\tZ", hit_x: 100, hit_y: 8, hit_request: 2)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "caret", value: [line, column]}}}
    assert {line, column} == {0, 4}
    render(ui, View, text: "", hit_x: 100, hit_y: 8, hit_request: 3)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "caret", value: [line, column]}}}
    assert {line, column} == {0, 0}
  end

  test "caret queries follow soft wrapping on plain and tabbed text", %{ui: ui} do
    for {text, offset, request, width} <- [{"abcdZ", 3, 1, 30}, {"abcd\tZ", 5, 2, 40}] do
      render(ui, View,
        text: text,
        width: width,
        wrap: true,
        hit_x: 1,
        hit_y: 28,
        hit_request: request
      )

      settle(ui)
      assert_receive {:gpui, ^ui, {:event, %{event: "caret", value: [line, column]}}}
      assert {line, column} == {0, offset}
    end
  end

  test "Select All retains raw Unicode and tabs, and dragging selects one tab", %{ui: ui} do
    for text <- ["ab\tZ", "é\tZ", "é\tZ", "👩🏽‍💻\tZ"] do
      render(ui, View, text: text)
      settle(ui)
      click(ui, "code")
      press(ui, "cmd-a")
      assert_receive {:gpui, ^ui, {:event, %{event: "selected", value: ^text}}}
    end

    render(ui, View, text: "ab\tZ")
    settle(ui)
    press(ui, :escape)
    drag(ui, {16, 30}, {16, 8})
    drag(ui, {16, 8}, {32, 8})
    assert_receive {:gpui, ^ui, {:event, %{event: "selected", value: "\t"}}}
  end

  test "tab size changes the stop while link offsets retain source indices", %{ui: ui} do
    alias GPUI.Text.{Position, Range, RichRun}
    run = RichRun.new(Range.new(Position.new(0, 2), Position.new(0, 3)), link: "marker")

    for {tab_size, x} <- [{2, 18}, {4, 34}] do
      render(ui, View, text: "a\tZ", runs: [run], selectable: false, tab_size: tab_size)
      settle(ui)
      drag(ui, {x, 30}, {x, 8})
      click(ui, {x, 8})
      assert_receive {:gpui, ^ui, {:event, %{event: "activated", value: "marker"}}}
    end
  end

  test "a trailing tab hangs before a soft wrap", %{ui: ui} do
    alias GPUI.Text.{Position, Range, RichRun}
    run = RichRun.new(Range.new(Position.new(0, 5), Position.new(0, 6)), link: "wrapped")
    render(ui, View, text: "abcd\tZ", width: 40, wrap: true, runs: [run], selectable: false)
    settle(ui)
    assert_in_delta bounds(ui, "code").height, 40, 0.1
    drag(ui, {4, 60}, {4, 28})
    click(ui, {4, 28})
    assert_receive {:gpui, ^ui, {:event, %{event: "activated", value: "wrapped"}}}
  end

  test "Find reveals original UTF-16 ranges after a tab and repeats after panning", %{ui: ui} do
    text = "ab\t" <> String.duplicate("x", 300) <> "target"
    attrs = [text: text, horizontal: true, reveal_start: 303, reveal_end: 309, request: 1]
    render(ui, View, attrs)
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "scrolled", value: [left, _, _, _]}}}
                   when left > 1000

    scroll(ui, "code", delta: {10000, 0})
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "scrolled", value: [left, _, _, _]}}}
                   when left == 0

    render(ui, View, Keyword.put(attrs, :request, 2))
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "scrolled", value: [left, _, _, _]}}}
                   when left > 1000
  end
end
