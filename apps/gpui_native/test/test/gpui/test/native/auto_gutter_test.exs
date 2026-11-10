defmodule GPUI.Test.Native.AutoGutterTest do
  use GPUI.Test, native: [size: {360, 240}]

  defmodule View do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      ~GPUI"""
      <UI.virtual_collection id="preview" label="Preview" measure_all={true}
        scrollbar={Map.get(assigns, :scrollbar, true)} scrollbar_gutter={Map.get(assigns, :gutter, "auto")}
        style={[width: {:px, 300.0}, height: {:px, Map.get(assigns, :viewport, 120) * 1.0}]}>
        <UI.virtual_item id="document" revision={assigns.height}>
          <div class="flex flex-col w-full" style={[height: {:px, assigns.height * 1.0}]}>
            <UI.rich_text id="probe" label="Document" text="Orchard" runs={[]} class="w-full h-5" />
          </div>
        </UI.virtual_item>
      </UI.virtual_collection>
      """
    end
  end

  test "automatic gutter appears for overflow and disappears after content shrinks", %{ui: ui} do
    render(ui, View, height: 20)
    settle(ui)
    assert_in_delta bounds(ui, "probe").width, 300, 0.1
    render(ui, View, height: 400)
    settle(ui)
    assert_in_delta bounds(ui, "probe").width, 290, 0.1
    render(ui, View, height: 20)
    settle(ui)
    assert_in_delta bounds(ui, "probe").width, 300, 0.1
  end

  test "viewport size changes update the lane without an application callback", %{ui: ui} do
    render(ui, View, height: 80, viewport: 120)
    settle(ui)
    assert_in_delta bounds(ui, "probe").width, 300, 0.1
    render(ui, View, height: 80, viewport: 60)
    settle(ui)
    assert_in_delta bounds(ui, "probe").width, 290, 0.1
    render(ui, View, height: 80, viewport: 200)
    settle(ui)
    assert_in_delta bounds(ui, "probe").width, 300, 0.1
  end

  test "stable gutters retain existing short-list layout and disabled scrollbars use full width",
       %{ui: ui} do
    render(ui, View, height: 20, gutter: "stable")
    settle(ui)
    assert_in_delta bounds(ui, "probe").width, 290, 0.1
    render(ui, View, height: 400, scrollbar: false)
    settle(ui)
    assert_in_delta bounds(ui, "probe").width, 300, 0.1
  end
end
