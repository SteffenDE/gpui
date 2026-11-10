defmodule GPUI.Test.Native.TabStopTest do
  use GPUI.Test, native: [size: {320, 160}]

  defmodule View do
    use GPUI.View

    @impl true
    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col gap-2">
        <button id="before" focus_request={assigns.request} phx-focus="before-focus" phx-click="before-click">Before</button>
        <button id="roving" tab_stop={assigns.tab_stop} phx-focus="roving-focus" phx-click="roving-click">Roving</button>
        <button id="after" phx-focus="after-focus" phx-click="after-click">After</button>
      </div>
      """
    end
  end

  defmodule OutlineView do
    use GPUI.View

    @impl true
    def render(_assigns) do
      ~GPUI"""
      <div class="flex flex-col gap-2 p-4">
        <button id="outline-before" focus_request={1} phx-focus="outline-before-focus" phx-click="before-click">Before</button>
        <div id="outlined" accessibility_role="tab" phx-focus="outline-focus" phx-blur="outline-blur" phx-click="outline-click"
          style={[focus_outline_color: {:rgb, 0x6BA5FB}, focus_outline_mode: :tab, focus_outline_width: {:px, 2.0}, focus_outline_offset: {:px, 1.0}, focus_outline_radius: {:px, 6.0}]}
          class="flex h-[26px] w-[100px] items-center px-2 rounded-md">
          <div id="outline-label"><text>State tab</text></div>
        </div>
      </div>
      """
    end
  end

  test "keyboard outlines preserve the control and text geometry", %{ui: ui} do
    render(ui, OutlineView, %{})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "outline-before-focus"}}}
    control = bounds(ui, "outlined")
    label = bounds(ui, "outline-label")
    press(ui, "tab")
    assert_receive {:gpui, ^ui, {:event, %{event: "outline-focus"}}}
    assert bounds(ui, "outlined") == control
    assert bounds(ui, "outline-label") == label
    press(ui, "home")
    assert bounds(ui, "outlined") == control
    assert bounds(ui, "outline-label") == label
  end

  defmodule OutlineOnlyView do
    use GPUI.View

    def render(_assigns) do
      ~GPUI"""
      <div class="flex flex-col gap-2 p-4">
        <button id="start" focus_request={1} phx-click="start">Start</button>
        <button id="disabled-outline" accessibility_disabled={true} phx-click="disabled-action" style={[focus_outline_color: {:rgb, 0x6BA5FB}]} class="w-[100px] h-[26px]">Disabled</button>
        <button id="outline-only" phx-click="outlined-action"
          style={[focus_outline_color: {:rgb, 0x6BA5FB}, focus_outline_mode: :tab, focus_outline_radius: {:px, 6.0}]}
          class="flex w-[100px] h-[26px] items-center px-2 rounded-md">
          <div id="outline-only-label"><text>Action</text></div>
        </button>
        <button id="finish" phx-click="finish">Finish</button>
      </div>
      """
    end
  end

  test "an outline-only button preserves layout and adds exactly one Tab stop", %{ui: ui} do
    render(ui, OutlineOnlyView, %{})
    settle(ui)
    control = bounds(ui, "outline-only")
    label = bounds(ui, "outline-only-label")
    press(ui, "tab")
    assert bounds(ui, "outline-only") == control
    assert bounds(ui, "outline-only-label") == label
    press(ui, "enter")
    assert_receive {:gpui, ^ui, {:event, %{event: "outlined-action"}}}
    press(ui, "tab")
    press(ui, "enter")
    assert_receive {:gpui, ^ui, {:event, %{event: "finish"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "disabled-action"}}}, 10
    refute_receive {:gpui, ^ui, {:event, %{event: "outlined-action"}}}, 10
  end

  test "excluded controls skip Tab but retain programmatic focus and activation", %{ui: ui} do
    render(ui, View, request: 1, tab_stop: false)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "before-focus"}}}
    press(ui, "tab")
    assert_receive {:gpui, ^ui, {:event, %{event: "after-focus"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "roving-focus"}}}, 10
    focus(ui, "roving")
    assert_receive {:gpui, ^ui, {:event, %{event: "roving-focus"}}}
    press(ui, "enter")
    assert_receive {:gpui, ^ui, {:event, %{event: "roving-click"}}}
  end

  test "rerendering can restore a control to Tab order", %{ui: ui} do
    render(ui, View, request: 1, tab_stop: false)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "before-focus"}}}
    render(ui, View, request: 2, tab_stop: true)
    settle(ui)
    press(ui, "tab")
    assert_receive {:gpui, ^ui, {:event, %{event: "roving-focus"}}}
  end
end
