defmodule GPUI.Test.Native.BoxShadowTest do
  use GPUI.Test, native: [size: {320, 120}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div class="relative w-full h-full">
        <button id="body" phx-click="clicked" class={"absolute left-[10px] top-[10px] w-[100px] h-[28px] p-0 rounded-md bg-[#1a1c20] " <> if(assigns.decorate, do: "shadow-md ring-1 ring-[#d7dbe1]/10", else: "")}>
          <div id="label"><text>Help</text></div>
        </button>
      </div>
      """
    end
  end

  defmodule InvalidView do
    use GPUI.View
    def render(assigns), do: ~GPUI'<div style={[box_shadow: assigns.layers]} class="w-10 h-10" />'
  end

  for {name, layers} <- [
        {"negative blur", [{0, 4, -1, 0, {:rgb, 0}}]},
        {"malformed layer", [{0, 4, 6}]},
        {"unrepresentable offset", [{1.0e300, 4, 6, 0, {:rgb, 0}}]}
      ] do
    @tag layers: layers, capture_log: true
    test "rejects #{name} before painting", %{ui: ui, layers: layers} do
      error =
        assert_raise GPUI.Test.NativeError, fn -> render(ui, InvalidView, layers: layers) end

      assert Exception.message(error) =~ "badarg"
    end
  end

  test "shadow and ring painting preserve layout and the original hit area", %{ui: ui} do
    render(ui, View, decorate: false)
    settle(ui)
    body = bounds(ui, "body")
    label = bounds(ui, "label")
    render(ui, View, decorate: true)
    settle(ui)
    assert bounds(ui, "body") == body
    assert bounds(ui, "label") == label
    click(ui, {9.5, 24})
    refute_receive {:gpui, ^ui, {:event, %{event: "clicked"}}}, 10
    click(ui, {10.5, 24})
    assert_receive {:gpui, ^ui, {:event, %{event: "clicked"}}}
    render(ui, View, decorate: false)
    settle(ui)
    assert bounds(ui, "body") == body
  end
end
