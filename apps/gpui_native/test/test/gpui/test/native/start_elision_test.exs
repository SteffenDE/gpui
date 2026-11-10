defmodule GPUI.Test.Native.StartElisionTest do
  use GPUI.Test, native: [size: {360, 220}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div id="path" style={[width: {:px, assigns.width * 1.0}]} class="flex min-w-0 text-[13px] leading-[20px]">
        <div id="directory" class="flex min-w-0 shrink">
          <text truncate_start={true} class="min-w-0 shrink whitespace-nowrap">{assigns.directory}</text>
        </div>
        <div id="basename" class="shrink-0"><text>garden.ex</text></div>
      </div>
      """
    end
  end

  test "directory text yields width to a preserved basename and reflows on resize", %{ui: ui} do
    directory = String.duplicate("long_directory/", 12)
    render(ui, View, width: 260, directory: directory)
    settle(ui)
    large = bounds(ui, "directory")
    base = bounds(ui, "basename")
    assert_in_delta base.x + base.width, 260, 1
    render(ui, View, width: 140, directory: directory)
    settle(ui)
    small = bounds(ui, "directory")
    narrow_base = bounds(ui, "basename")
    assert_in_delta large.width - small.width, 120, 1
    assert_in_delta narrow_base.width, base.width, 0.1
    assert_in_delta narrow_base.x + narrow_base.width, 140, 1
    assert_in_delta bounds(ui, "path").height, 20, 0.1
  end

  test "Unicode clusters and empty prefixes keep single-line layout", %{ui: ui} do
    for directory <- [String.duplicate("👩🏽‍💻/é/你好/", 10), ""] do
      render(ui, View, width: 140, directory: directory)
      settle(ui)
      assert_in_delta bounds(ui, "path").height, 20, 0.1
      assert bounds(ui, "basename").width > 0
    end
  end
end
