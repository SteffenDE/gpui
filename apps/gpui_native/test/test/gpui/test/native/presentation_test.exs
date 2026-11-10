defmodule GPUI.Test.Native.PresentationTest do
  use GPUI.Test, native: [size: {360, 240}]

  defmodule View do
    use GPUI.View

    alias GPUI.UI

    @impl GPUI.View
    def render(_assigns) do
      ~GPUI"""
      <div class="flex flex-col gap-2 w-full h-full">
        <UI.paint
          id="paint-surface"
          commands={[
            %{type: :rect, x: 0, y: 0, width: 0, height: 0, color: 0xFFFFFFFF},
            %{type: :line, x1: -20, y1: 20, x2: 180, y2: 20, width: 2, color: 0xFFFFFFFF}
          ]}
          class="w-[120px] h-[40px] overflow-hidden"
        />

        <UI.frost
          id="opaque-frost"
          fallback="translucent"
          opacity={0.25}
          reduced_transparency={true}
          class="w-[140px] h-[36px]"
        >
          <text>Opaque fallback</text>
        </UI.frost>

        <UI.edge_fade
          id="bounded-fades"
          edges={[:top, :bottom]}
          size={16}
          class="w-[160px] h-[80px]"
        >
          <scroll id="fade-scroll" class="w-full h-full">
            <text>Scrolling remains an explicit child.</text>
          </scroll>
        </UI.edge_fade>
      </div>
      """
    end
  end

  test "presentation surfaces keep their declared layout bounds", %{ui: ui} do
    render(ui, View, %{})

    assert %{width: 120.0, height: 40.0} = bounds(ui, "paint-surface")
    assert %{width: 140.0, height: 36.0} = bounds(ui, "opaque-frost")
    assert %{width: 160.0, height: 80.0} = bounds(ui, "bounded-fades")
  end

  defmodule FractionView do
    use GPUI.View

    def render(_assigns) do
      ~GPUI"""
      <div class="relative w-full h-full">
        <div id="fraction-box" class="absolute top-1/2 left-1/2 w-1/2 h-1/2" />
        <div id="negative-offset" class="absolute -top-1/4 left-1/4 w-1/4 h-1/2" />
      </div>
      """
    end
  end

  test "fraction sizes and signed positions decode from canonical list payloads", %{ui: ui} do
    render(ui, FractionView, %{})
    settle(ui)
    assert %{x: 180.0, y: 120.0, width: 180.0, height: 120.0} = bounds(ui, "fraction-box")
    assert %{x: 90.0, y: -60.0, width: 90.0, height: 120.0} = bounds(ui, "negative-offset")
    resize(ui, {720, 480})
    settle(ui)
    assert %{x: 360.0, y: 240.0, width: 360.0, height: 240.0} = bounds(ui, "fraction-box")
  end

  defmodule ButtonStylesView do
    use GPUI.View

    def render(_assigns) do
      ~GPUI"""
      <div class="flex flex-col items-start gap-2 w-full h-full">
        <button id="default-button" class="flex"><div id="default-content" class="w-5 h-[10px]" /></button>
        <button id="padded-button" class="flex px-4 py-1 rounded-none"><div id="padded-content" class="w-5 h-[10px]" /></button>
        <button id="zero-button" class="flex p-0 rounded-none"><div id="zero-content" class="w-5 h-[10px]" /></button>
      </div>
      """
    end
  end

  test "explicit button styles override native defaults while unstyled buttons retain them", %{
    ui: ui
  } do
    render(ui, ButtonStylesView, %{})
    settle(ui)
    assert %{width: 40.0, height: 22.0} = bounds(ui, "default-button")
    assert %{width: 52.0, height: 18.0} = bounds(ui, "padded-button")
    assert %{width: 20.0, height: 10.0} = bounds(ui, "zero-button")
    assert bounds(ui, "padded-content").x - bounds(ui, "padded-button").x == 16.0
    assert bounds(ui, "zero-content").x == bounds(ui, "zero-button").x
  end
end
