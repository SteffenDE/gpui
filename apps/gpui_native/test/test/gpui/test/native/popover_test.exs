defmodule GPUI.Test.Native.PopoverTest do
  use GPUI.Test, native: [size: {320, 220}]

  defmodule View do
    use GPUI.View
    alias GPUI.UI.Overlay

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col w-full h-full gap-2">
        <text_input id="outside-popover" value="" focus_request={1} capture_keys={["escape"]} phx-keystroke="outside-key" phx-change="typed" class="w-[200px] h-8" />
        <Overlay.popover id="initial-popover" label="Initially open" open={true} open_on_hover={Map.get(assigns, :hover, false)} focus_on_open={assigns.focus_on_open} anchor="top_left" phx-change="popover-open">
          <:trigger><div class="w-[120px] h-8">Open</div></:trigger>
          <:content><text class="w-[140px] h-8">Popover content</text></:content>
        </Overlay.popover>
      </div>
      """
    end
  end

  defmodule EscapeReturnView do
    use GPUI.View
    alias GPUI.UI.Overlay

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col gap-2">
        <button id="return-before" phx-focus="return-before-focused" focus_request={1}>Before</button>
        <Overlay.popover id="return-popover" label="History" open={assigns.open} focus_on_open={false} phx-change="return-open">
          <:trigger><Overlay.tooltip id="return-hint" keyboard={true}><:trigger><div id="return-trigger" class="w-[120px] h-8">History</div></:trigger><:content>History</:content></Overlay.tooltip></:trigger>
          <:content>{if assigns.open, do: inside(), else: []}</:content>
        </Overlay.popover>
        <button id="return-after" phx-focus="return-after-focused">After</button>
      </div>
      """
    end

    defp inside do
      ~GPUI'<button id="return-inside" phx-focus="return-inside-focused" focus_request={1}>Inside</button>'
    end
  end

  test "Escape returns an interactive popover to its trigger before the next Tab", %{ui: ui} do
    render(ui, EscapeReturnView, open: false)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "return-before-focused"}}}
    click(ui, "return-trigger")
    assert_receive {:gpui, ^ui, {:event, %{event: "return-open", value: true}}}
    render(ui, EscapeReturnView, open: true)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "return-inside-focused"}}}
    press(ui, "escape")
    assert_receive {:gpui, ^ui, {:event, %{event: "return-open", value: false}}}
    render(ui, EscapeReturnView, open: false)
    settle(ui)
    advance(ui, 32)
    press(ui, "tab")
    assert_receive {:gpui, ^ui, {:event, %{event: "return-after-focused"}}}
  end

  defmodule ClickVirtualView do
    use GPUI.View
    alias GPUI.UI.Overlay

    def render(_assigns) do
      ~GPUI"""
      <div class="w-full h-full">
        <text_input id="virtual-outside" value="" focus_request={1} capture_keys={["escape"]} phx-keystroke="outside-key" />
        <Overlay.popover id="click-virtual" label="Copied batches" open={true} appearance={false} anchor_x={20} anchor_y={40} anchor_width={0} anchor_height={0} phx-change="click-virtual-open">
          <:trigger><div /></:trigger>
          <:content><button id="virtual-action" class="w-[100px] h-8">Action</button></:content>
        </Overlay.popover>
      </div>
      """
    end
  end

  test "Escape dismisses a click-opened virtual popover and consumes the key", %{ui: ui} do
    render(ui, ClickVirtualView, %{})
    settle(ui)
    press(ui, "escape")
    assert_receive {:gpui, ^ui, {:event, %{event: "click-virtual-open", value: false}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "outside-key"}}}
  end

  defmodule HoverLinkView do
    use GPUI.View
    alias GPUI.UI.Overlay

    @impl true
    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col gap-2">
        <button id="hover-before" phx-focus="hover-before-focused" focus_request={1} phx-click="before-click">Before</button>
        <Overlay.popover id="profile-card" label="Profile" open={false} trigger_passthrough={Map.get(assigns, :passthrough, true)} open_on_hover={true} hover_delay={10000} focus_on_open={false} phx-change="profile-open">
          <:trigger><button id="profile-link" phx-focus="profile-focused" phx-click="profile-click">Profile link</button></:trigger>
          <:content><text>Profile card</text></:content>
        </Overlay.popover>
        <button id="hover-after" phx-focus="hover-after-focused" phx-click="after-click">After</button>
      </div>
      """
    end
  end

  test "hover cards preserve the child control's Tab order and Enter activation", %{ui: ui} do
    render(ui, HoverLinkView, %{})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "hover-before-focused"}}}
    press(ui, "tab")
    assert_receive {:gpui, ^ui, {:event, %{event: "profile-focused"}}}
    press(ui, "enter")
    assert_receive {:gpui, ^ui, {:event, %{event: "profile-click"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "profile-open"}}}, 10
    press(ui, "tab")
    assert_receive {:gpui, ^ui, {:event, %{event: "hover-after-focused"}}}
    press(ui, "shift-tab")
    assert_receive {:gpui, ^ui, {:event, %{event: "profile-focused"}}}
    click(ui, "profile-link")
    assert_receive {:gpui, ^ui, {:event, %{event: "profile-click"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "profile-open"}}}, 10
  end

  test "ordinary hover popovers retain their focusable trigger by default", %{ui: ui} do
    render(ui, HoverLinkView, passthrough: false)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "hover-before-focused"}}}
    press(ui, "tab")
    refute_receive {:gpui, ^ui, {:event, %{event: "profile-focused"}}}, 10
    press(ui, "enter")
    assert_receive {:gpui, ^ui, {:event, %{event: "profile-open", value: true}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "profile-click"}}}, 10
  end

  defmodule VirtualView do
    use GPUI.View
    alias GPUI.UI.Overlay

    def render(assigns) do
      ~GPUI"""
      <div class="w-full h-full">
        <Overlay.popover id="virtual-popover" label="Mention" open={assigns.open} open_on_hover={true} hover_delay={100} close_delay={200} focus_on_open={false} appearance={false} anchor="top_left" anchor_x={120} anchor_y={60} anchor_width={40} anchor_height={20} anchor_hovered={assigns.hovered} phx-change="virtual-open">
          <:trigger><div /></:trigger>
          <:content><div id="virtual-content" class="w-[100px] h-[40px]">Profile</div></:content>
        </Overlay.popover>
      </div>
      """
    end
  end

  test "virtual anchors open after hover delay and place content below the measured span", %{
    ui: ui
  } do
    render(ui, VirtualView, open: false, hovered: true)
    advance(ui, 99)
    refute_receive {:gpui, ^ui, {:event, %{event: "virtual-open"}}}, 10
    advance(ui, 2)
    assert_receive {:gpui, ^ui, {:event, %{event: "virtual-open", value: true}}}
    render(ui, VirtualView, open: true, hovered: true)
    settle(ui)
    box = bounds(ui, "virtual-content")
    assert_in_delta box.x, 120, 1
    assert_in_delta box.y, 84, 1
    assert_in_delta box.width, 100, 1
    press(ui, "escape")
    assert_receive {:gpui, ^ui, {:event, %{event: "virtual-open", value: false}}}
  end

  defmodule StackView do
    use GPUI.View
    alias GPUI.UI.Overlay

    def render(assigns) do
      ~GPUI"""
      <div class="w-full h-full">
        <Overlay.popover id="parent-preview" label="Parent" open={true} open_on_hover={true} focus_on_open={false} appearance={false} anchor="top_left" anchor_x={20} anchor_y={20} anchor_width={40} anchor_height={20} anchor_hovered={true} phx-change="parent-open">
          <:trigger><div /></:trigger>
          <:content><div class="w-[180px] h-[100px]">Parent preview</div></:content>
        </Overlay.popover>
        <Overlay.popover id="child-preview" label="Child" parent_popover="parent-preview" open={assigns.child_open} open_on_hover={true} focus_on_open={false} appearance={false} anchor="top_left" anchor_x={100} anchor_y={80} anchor_width={40} anchor_height={20} anchor_hovered={assigns.child_open} phx-change="child-open">
          <:trigger><div /></:trigger>
          <:content><button id="child-action" phx-click="child-action" class="w-[160px] h-[70px]">Child preview</button></:content>
        </Overlay.popover>
      </div>
      """
    end
  end

  test "Escape dismisses the most recently opened preview without closing its parent", %{ui: ui} do
    render(ui, StackView, child_open: false)
    advance(ui, 16)
    render(ui, StackView, child_open: true)
    advance(ui, 16)
    settle(ui)
    click(ui, {120, 160})
    assert_receive {:gpui, ^ui, {:event, %{event: "child-action"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "parent-open", value: false}}}, 20
    press(ui, "escape")
    assert_receive {:gpui, ^ui, {:event, %{event: "child-open", value: false}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "parent-open", value: false}}}, 20
    render(ui, StackView, child_open: false)
    settle(ui)
    press(ui, "escape")
    assert_receive {:gpui, ^ui, {:event, %{event: "parent-open", value: false}}}
  end

  test "a popover mounted open takes focus and Escape requests dismissal", %{ui: ui} do
    render(ui, View, %{focus_on_open: true})
    advance(ui, 16)
    settle(ui)
    press(ui, "escape")
    assert_receive {:gpui, ^ui, {:event, %{type: :change, event: "popover-open", value: false}}}
  end

  test "a popover mounted open can intentionally preserve the current input focus", %{ui: ui} do
    render(ui, View, %{focus_on_open: false})
    advance(ui, 16)
    settle(ui)
    type(ui, "garden")
    assert_receive {:gpui, ^ui, {:event, %{event: "typed", value: "garden"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "popover-open"}}}
  end

  for hover <- [false, true] do
    @tag hover: hover
    test "Escape dismisses a nonmodal popup (hover=#{hover}) before a focused input captures it",
         %{ui: ui, hover: hover} do
      render(ui, View, %{focus_on_open: false, hover: hover})
      advance(ui, 16)
      settle(ui)
      press(ui, "escape")
      assert_receive {:gpui, ^ui, {:event, %{type: :change, event: "popover-open", value: false}}}
      refute_receive {:gpui, ^ui, {:event, %{event: "outside-key", value: %{key: "escape"}}}}
    end
  end
end
