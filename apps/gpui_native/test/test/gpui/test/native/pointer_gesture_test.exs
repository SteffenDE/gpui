defmodule GPUI.Test.Native.PointerGestureTest do
  use GPUI.Test, native: [size: {320, 120}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div id="pointer-root" phx-click="parent-click" phx-window-pointer-position={if assigns.enabled, do: "window-move"} phx-window-pointer-up={if assigns.enabled, do: "window-up"} class="flex w-full h-full">
        <button id="gesture-button" accessibility_disabled={Map.get(assigns,:disabled,false)} phx-focus="button-focused" phx-pointer-down={if assigns.enabled, do: "pressed"} phx-click="activated" class="w-[70px] h-10">Drag</button>
        <text_input id="other-input" value="" phx-change="typed" class="w-[100px] h-10" />
      </div>
      """
    end
  end

  test "captured presses suppress mouse click and parent activation but preserve keyboard", %{
    ui: ui
  } do
    render(ui, View, %{enabled: true})
    settle(ui)
    click(ui, {30, 20})
    assert_receive {:gpui, ^ui, {:event, %{event: "pressed", value: [30.0, 20.0]}}}
    assert_receive {:gpui, ^ui, {:event, %{event: "window-up", value: [30.0, 20.0]}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "activated"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "parent-click"}}}
    focus(ui, "gesture-button")
    press(ui, "enter")
    assert_receive {:gpui, ^ui, {:event, %{event: "activated"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "pressed"}}}
  end

  test "window observers retain movement and release outside the pressed control and viewport", %{
    ui: ui
  } do
    render(ui, View, %{enabled: true})
    settle(ui)
    drag(ui, {30, 20}, {350, 140})
    assert_receive {:gpui, ^ui, {:event, %{event: "pressed", value: [30.0, 20.0]}}}
    assert_receive {:gpui, ^ui, {:event, %{event: "window-move", value: [350.0, 140.0]}}}
    assert_receive {:gpui, ^ui, {:event, %{event: "window-up", value: [350.0, 140.0]}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "activated"}}}
  end

  test "disabled pointer controls do not capture presses or activate", %{ui: ui} do
    render(ui, View, %{enabled: true, disabled: true})
    settle(ui)
    click(ui, {30, 20})
    assert_receive {:gpui, ^ui, {:event, %{event: "window-up", value: [30.0, 20.0]}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "pressed"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "activated"}}}
    render(ui, View, %{enabled: true, disabled: false})
    settle(ui)
    click(ui, {30, 20})
    assert_receive {:gpui, ^ui, {:event, %{event: "pressed", value: [30.0, 20.0]}}}
  end

  test "removing pointer callbacks restores ordinary clicking and input editing", %{ui: ui} do
    render(ui, View, %{enabled: true})
    settle(ui)
    render(ui, View, %{enabled: false})
    settle(ui)
    click(ui, "gesture-button")
    assert_receive {:gpui, ^ui, {:event, %{event: "activated"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "pressed"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "window-up"}}}
    click(ui, {110, 20})
    type(ui, "c")
    assert_receive {:gpui, ^ui, {:event, %{event: "typed", value: "c"}}}
  end
end
