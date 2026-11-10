defmodule GPUI.Test.Native.LayoutBoundsTest do
  use GPUI.Test, native: [size: {320, 120}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col w-full h-full">
        <div id="header" style={[preserve_layout_bounds: assigns.preserve]} class="flex w-full h-9 shrink-0 items-center border-b">
          <div class="flex h-full items-center">
            <button id="control" phx-click="clicked" style={[height: {:fraction, 2 / 3}]} class="w-6 shrink-0 p-0">A</button>
          </div>
        </div>
        <div class="flex w-full h-9 shrink-0 items-center border-b">
          <button id="sibling" phx-click="sibling" class="w-6 h-6 shrink-0 p-0">B</button>
        </div>
      </div>
      """
    end
  end

  test "fractional subtree bounds drive hit testing while siblings and default rendering stay snapped",
       %{ui: ui} do
    render(ui, View, preserve: false)
    settle(ui)
    snapped = bounds(ui, "control")
    sibling = bounds(ui, "sibling")
    refute_in_delta snapped.y, 35 / 6, 0.001
    click(ui, {12, snapped.y - 0.05})
    refute_receive {:gpui, ^ui, {:event, %{event: "clicked"}}}, 10

    render(ui, View, preserve: true)
    settle(ui)
    control = bounds(ui, "control")
    assert_in_delta control.y, 35 / 6, 0.001
    assert_in_delta control.height, 70 / 3, 0.001
    assert bounds(ui, "sibling") == sibling
    click(ui, {12, snapped.y - 0.05})
    assert_receive {:gpui, ^ui, {:event, %{event: "clicked"}}}

    render(ui, View, preserve: false)
    settle(ui)
    assert bounds(ui, "control") == snapped
  end
end
