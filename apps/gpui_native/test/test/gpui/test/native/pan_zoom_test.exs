defmodule GPUI.Test.Native.PanZoomTest do
  use GPUI.Test, native: [size: {400, 240}]

  defmodule CanvasView do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <GPUI.UI.pan_zoom id="canvas" label="Canvas" content_width={160} content_height={80}
        pan_x={Map.get(assigns, :x, 0.0)} pan_y={Map.get(assigns, :y, 0.0)} scale={Map.get(assigns, :scale, 1.0)}
        reset_request={Map.get(assigns, :reset, 0)} zoomable={Map.get(assigns, :zoomable, true)}
        phx-change="transform" phx-click="dismiss">
        <div id="content" class="w-full h-full bg-[#8250df]" />
      </GPUI.UI.pan_zoom>
      """
    end
  end

  defmodule ModalView do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <GPUI.UI.Overlay.dialog id="full-dialog" title="Image" open={assigns.open} fullscreen={true}
        width={100} margin_top={100} show_title={false} close_button={false} phx-change="dialog">
        <:content>
          <div id="full-canvas" class="w-full h-full" />
        </:content>
      </GPUI.UI.Overlay.dialog>
      """
    end
  end

  test "wheels pan the image and stale controlled snapshots do not lose local movement", %{ui: ui} do
    render(ui, CanvasView, %{})
    assert %{x: 120.0, y: 80.0, width: 160.0, height: 80.0} = bounds(ui, "content")
    scroll(ui, "canvas", delta: {15, -20})
    assert_receive {:gpui, ^ui, {:event, %{event: "transform", value: [15.0, -20.0, 1.0]}}}
    assert %{x: 135.0, y: 60.0} = bounds(ui, "content")
    render(ui, CanvasView, %{})
    assert %{x: 135.0, y: 60.0} = bounds(ui, "content")
    render(ui, CanvasView, %{x: 15.0, y: -20.0})
    assert %{x: 135.0, y: 60.0} = bounds(ui, "content")
    render(ui, CanvasView, %{x: 0.0, y: 0.0, scale: 2.0})
    assert %{x: 40.0, y: 40.0, width: 320.0, height: 160.0} = bounds(ui, "content")
    render(ui, CanvasView, %{reset: 1})
    assert %{x: 120.0, y: 80.0, width: 160.0, height: 80.0} = bounds(ui, "content")
  end

  test "captured drags pan without dismissing, and an unmoved outside click dismisses", %{ui: ui} do
    render(ui, CanvasView, %{})
    drag(ui, {200, 120}, {240, 150})
    assert_receive {:gpui, ^ui, {:event, %{event: "transform", value: [40.0, 30.0, 1.0]}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "dismiss"}}}, 10
    click(ui, {10, 10})
    assert_receive {:gpui, ^ui, {:event, %{event: "dismiss"}}}
  end

  test "small motions remain clicks and a disabled zoomable canvas still dismisses", %{ui: ui} do
    render(ui, CanvasView, %{})
    drag(ui, {10, 10}, {12, 11})
    assert_receive {:gpui, ^ui, {:event, %{event: "dismiss"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "transform"}}}, 10
    render(ui, CanvasView, %{zoomable: false})
    scroll(ui, "canvas", delta: {20, 20})
    refute_receive {:gpui, ^ui, {:event, %{event: "transform"}}}, 10
    click(ui, {10, 10})
    assert_receive {:gpui, ^ui, {:event, %{event: "dismiss"}}}
  end

  test "native pinch preserves the anchored content point and clamps zoom", %{ui: ui} do
    render(ui, CanvasView, %{x: 20.0, y: -10.0})
    pinch(ui, "canvas", delta: 0.0, phase: :started)
    pinch(ui, "canvas", delta: 0.5)
    assert_receive {:gpui, ^ui, {:event, %{event: "transform", value: [30.0, -15.0, 1.5]}}}
    assert %{x: 110.0, y: 45.0, width: 240.0, height: 120.0} = bounds(ui, "content")
    pinch(ui, "canvas", delta: 100.0)
    assert_receive {:gpui, ^ui, {:event, %{event: "transform", value: [_, _, 10.0]}}}
    pinch(ui, "canvas", delta: 0.0, phase: :ended)
    double_click(ui, "canvas")

    assert_receive {:gpui, ^ui, {:event, %{event: "transform", value: [x, y, 1.0]}}}
                   when x == 0 and y == 0
  end

  test "cancelled pinches restore their starting view", %{ui: ui} do
    render(ui, CanvasView, %{})
    pinch(ui, "canvas", delta: 0.0, phase: :started)
    pinch(ui, "canvas", delta: 0.5)
    assert_receive {:gpui, ^ui, {:event, %{event: "transform", value: [_, _, 1.5]}}}
    pinch(ui, "canvas", delta: 0.0, phase: :cancelled)

    assert_receive {:gpui, ^ui, {:event, %{event: "transform", value: [x, y, 1.0]}}}
                   when x == 0 and y == 0

    assert %{width: 160.0, height: 80.0} = bounds(ui, "content")
  end

  test "double-click zooms the initial view and resets an already transformed view", %{ui: ui} do
    render(ui, CanvasView, %{})
    double_click(ui, "canvas")

    assert_receive {:gpui, ^ui, {:event, %{event: "transform", value: [x, y, 2.0]}}}
                   when x == 0 and y == 0

    assert %{width: 320.0, height: 160.0} = bounds(ui, "content")
    double_click(ui, "canvas")

    assert_receive {:gpui, ^ui, {:event, %{event: "transform", value: [x, y, 1.0]}}}
                   when x == 0 and y == 0

    assert %{width: 160.0, height: 80.0} = bounds(ui, "content")
  end

  test "fullscreen dialogs cover the viewport and resize independently of width/margin props", %{
    ui: ui
  } do
    render(ui, ModalView, %{open: true})
    advance(ui, 250)
    assert %{x: +0.0, y: +0.0, width: 400.0, height: 240.0} = bounds(ui, "full-canvas")
    resize(ui, {500, 300})
    assert %{x: +0.0, y: +0.0, width: 500.0, height: 300.0} = bounds(ui, "full-canvas")
    press(ui, :escape)
    assert_receive {:gpui, ^ui, {:event, %{event: "dialog", value: false}}}
  end
end
