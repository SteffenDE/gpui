defmodule GPUI.Test.Native.ScrollTest do
  use GPUI.Test, native: [size: {320, 240}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      assigns = Map.merge(%{height: 200, scrollbar: false}, assigns)

      ~GPUI"""
      <scroll id="pane" scrollbar={assigns.scrollbar} scroll_top={assigns.top} scroll_request={assigns.request} phx-scroll="scrolled" phx-bounds-change="pane_bounds" class="w-[200px] h-[100px]">
        <div class="flex flex-col w-full shrink-0" style={[height: {:px, assigns.height * 1.0}]}>
          <div class="h-[50px] shrink-0" />
          <button id="middle" phx-click="middle" class="h-5 shrink-0">Middle</button>
          <div class="shrink-0" style={[height: {:px, max(0, assigns.height - 90) * 1.0}]} />
          <button id="last" phx-click="last" class="h-5 shrink-0">Last</button>
        </div>
      </scroll>
      """
    end
  end

  defmodule HorizontalView do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <scroll id="horizontal" scroll_horizontal={true} scroll_left={assigns.left} scroll_request={assigns.request} scrollbar={assigns.scrollbar} phx-scroll="horizontal-scroll" class="flex w-[200px] h-[100px]">
        <div class="flex w-[500px] shrink-0 h-full"><div class="w-[100px] shrink-0" /><button id="horizontal-middle" phx-click="middle" class="w-[50px] h-8 shrink-0">Middle</button><div class="w-[300px] shrink-0" /><button id="horizontal-last" phx-click="last" class="w-[50px] h-8 shrink-0">Last</button></div>
      </scroll>
      """
    end
  end

  test "horizontal scroll requests, wheel input and track clicks preserve independent left offsets",
       %{ui: ui} do
    render(ui, HorizontalView, left: 0, request: 0, scrollbar: false)
    settle(ui)
    assert_in_delta bounds(ui, "horizontal-middle").x, 100, 0.1

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "horizontal-scroll", value: [left, 200.0, 300.0]}}}
                   when left == 0

    render(ui, HorizontalView, left: 80, request: 1, scrollbar: false)
    settle(ui)
    assert_in_delta bounds(ui, "horizontal-middle").x, 20, 0.1
    render(ui, HorizontalView, left: 200, request: 1, scrollbar: false)
    settle(ui)
    assert_in_delta bounds(ui, "horizontal-middle").x, 20, 0.1
    scroll(ui, "horizontal", delta: {-50, 0})
    settle(ui)
    assert_in_delta bounds(ui, "horizontal-middle").x, -30, 0.1
    render(ui, HorizontalView, left: 0, request: 2, scrollbar: true)
    settle(ui)
    click(ui, {190, 95})
    settle(ui)
    assert bounds(ui, "horizontal-middle").x < 0
  end

  test "scroll requests move content, clamp to its extent, and preserve offsets on rerender", %{
    ui: ui
  } do
    render(ui, View, top: 40, request: 0)
    settle(ui)
    assert_in_delta bounds(ui, "middle").y, 50, 0.1

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{event: "pane_bounds", value: %{x: x, y: y, width: 200.0, height: 100.0}}}}
                   when x == 0 and y == 0

    assert_receive {:gpui, ^ui, {:event, %{event: "scrolled", value: [top, 100.0, 100.0]}}}
                   when top == 0

    render(ui, View, top: 40, request: 1)
    settle(ui)
    assert_in_delta bounds(ui, "middle").y, 10, 0.1
    assert_receive {:gpui, ^ui, {:event, %{event: "scrolled", value: [40.0, 100.0, 100.0]}}}
    render(ui, View, top: 80, request: 1)
    settle(ui)
    assert_in_delta bounds(ui, "middle").y, 10, 0.1
    render(ui, View, top: 500, request: 2)
    settle(ui)
    assert_in_delta bounds(ui, "last").y, 80, 0.1
    assert_receive {:gpui, ^ui, {:event, %{event: "scrolled", value: [100.0, 100.0, 100.0]}}}
    click(ui, "last")
    assert_receive {:gpui, ^ui, {:event, %{event: "last"}}}
    scroll(ui, "pane", delta: {0, 40})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "scrolled", value: [60.0, 100.0, 100.0]}}}
    render(ui, View, top: 0, request: 2)
    settle(ui)
    assert_in_delta bounds(ui, "last").y, 120, 0.1
    refute_receive {:gpui, ^ui, {:event, %{event: "pane_bounds"}}}, 10
  end

  test "scrollbars reserve a gutter only for overflow and support track clicks and captured drags",
       %{ui: ui} do
    render(ui, View, top: 0, request: 0, scrollbar: true, height: 1000)
    settle(ui)
    assert_in_delta bounds(ui, "middle").width, 190, 0.1
    click(ui, {195, 90})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "scrolled", value: [80.0, 100.0, 900.0]}}}
    render(ui, View, top: 900, request: 1, scrollbar: true, height: 1000)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "scrolled", value: [900.0, 100.0, 900.0]}}}
    click(ui, {195, 10})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "scrolled", value: [820.0, 100.0, 900.0]}}}
    drag(ui, {195, 82}, {195, 0})
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "scrolled", value: [top, 100.0, 900.0]}}}
                   when top == 0

    click(ui, "middle")
    assert_receive {:gpui, ^ui, {:event, %{event: "middle"}}}
    render(ui, View, top: 0, request: 2, scrollbar: true, height: 90)
    settle(ui)
    assert_in_delta bounds(ui, "middle").width, 200, 0.1
    render(ui, View, top: 0, request: 3, scrollbar: false, height: 1000)
    settle(ui)
    assert_in_delta bounds(ui, "middle").width, 200, 0.1
  end
end
