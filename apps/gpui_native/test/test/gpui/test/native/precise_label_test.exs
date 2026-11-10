defmodule GPUI.Test.Native.PreciseLabelTest do
  use GPUI.Test, native: [size: {360, 200}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col items-start text-[9px] leading-[13px] font-medium">
        <div id="precise-label" style={[preserve_layout_bounds: true]} class="flex h-[13px] px-[3px]">
          <text style={[preserve_layout_bounds: true]} class="whitespace-nowrap">{assigns.label}</text>
        </div>
        <div id="precise-ellipsis" style={[preserve_layout_bounds: true]} class="flex h-[13px] px-[3px]">
          <text style={[preserve_layout_bounds: true]} class="truncate">{assigns.label}</text>
        </div>
        <div id="narrow-ellipsis" class="flex w-5 min-w-0 h-[13px]">
          <text style={[preserve_layout_bounds: true]} class="min-w-0 truncate">{String.duplicate(assigns.label, 20)}</text>
        </div>
        <div id="ordinary-label" class="flex h-[13px] px-[3px]"><text>{assigns.label}</text></div>
        <div id="multiline-label" style={[preserve_layout_bounds: true]}><text style={[preserve_layout_bounds: true]} class="whitespace-nowrap">{"Q\nQ"}</text></div>
      </div>
      """
    end
  end

  test "explicit precise labels retain fractional widths without changing ordinary labels", %{
    ui: ui
  } do
    render(ui, View, label: "Q")
    settle(ui)
    precise = bounds(ui, "precise-label")
    ordinary = bounds(ui, "ordinary-label")
    assert precise.width > 6
    assert abs(precise.width - Float.round(precise.width)) > 0.01
    assert ordinary.width == Float.ceil(precise.width)
    assert_in_delta bounds(ui, "precise-ellipsis").width, precise.width, 0.01
    assert_in_delta bounds(ui, "narrow-ellipsis").height, 13, 0.01
    assert_in_delta bounds(ui, "narrow-ellipsis").width, 20, 0.01
    assert_in_delta precise.height, 13, 0.01
    render(ui, View, label: "⌘K")
    settle(ui)
    assert bounds(ui, "precise-label").width > precise.width
  end

  test "explicit line breaks retain multiline layout", %{ui: ui} do
    render(ui, View, label: "Q")
    settle(ui)
    assert_in_delta bounds(ui, "multiline-label").height, 26, 0.1
  end
end
