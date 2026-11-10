defmodule GPUI.Test.Native.MotionRepeatTest do
  use GPUI.Test, native: [size: {240, 120}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div class="relative w-full h-full">
        <div id="pulse" motion_request={1} motion_duration={2000} motion_repeat={true} motion_alternate={true} motion_to_opacity={0.5} class="absolute top-1 right-1 w-2 h-2 bg-[#ff0000]" />
        <button id="moving" accessibility_label="Moving action" phx-click="activate"
          motion_request={assigns.request} motion_duration={1000} motion_repeat={true}
          motion_alternate={assigns.alternate} motion_easing="linear" motion_to_x={80}
          class="w-8 h-8 bg-[#00ff00]" />
        <svg id="spinner" spin_duration={1000} class="w-4 h-4" data="<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path fill='currentColor' d='M8 0H16V8H8z'/></svg>" />
      </div>
      """
    end
  end

  test "repeated motion keeps progressing beyond the first cycle", %{ui: ui} do
    render(ui, View, request: 1, alternate: false)
    advance(ui, 250)
    assert %{x: 228.0, y: 4.0} = bounds(ui, "pulse")
    x = bounds(ui, "moving").x
    assert_in_delta x, 20, 10
    advance(ui, 1000)
    assert_in_delta bounds(ui, "moving").x, x, 3
    click(ui, "moving")
    assert_receive {:gpui, ^ui, {:event, %{type: :click, event: "activate"}}}
  end

  test "alternate motion returns toward its starting position", %{ui: ui} do
    render(ui, View, request: 1, alternate: true)
    advance(ui, 250)
    assert %{x: 228.0, y: 4.0} = bounds(ui, "pulse")
    x = bounds(ui, "moving").x
    assert_in_delta x, 40, 10
    advance(ui, 500)
    assert_in_delta bounds(ui, "moving").x, x, 4
  end
end
