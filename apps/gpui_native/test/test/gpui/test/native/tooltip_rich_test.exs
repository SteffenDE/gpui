defmodule GPUI.Test.Native.TooltipRichTest do
  use GPUI.Test, native: [size: {400, 200}]

  defmodule View do
    use GPUI.View
    alias GPUI.UI.Overlay

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col items-start">
        <button id="before-rich" focus_request={1}>Before</button>
        <Overlay.tooltip id="rich-hint" keyboard={true} appearance={false} placement="top_start" offset={6} delay={40}>
          <:trigger><button id="rich-trigger" phx-click="rich-clicked" class="w-[120px] h-8">Show card</button></:trigger>
          <:content><div id="rich-body" phx-bounds-change="rich-body-bounds" class="flex items-center gap-1.5 px-2.5 py-1.5 text-[12px]">
            <svg data="<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12'><path fill='currentColor' d='M0 0h12v12H0z'/></svg>" class="w-3 h-3" />
            <text>{assigns.title}</text><text class="text-[#abaeb4]"> · context</text>
          </div></:content>
        </Overlay.tooltip>
      </div>
      """
    end
  end

  test "keyboard tooltip renders rich children and refreshes a visible label", %{ui: ui} do
    render(ui, View, title: "Initial")
    press(ui, "tab")
    advance(ui, 50)
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{type: :bounds, event: "rich-body-bounds", value: %{width: first}}}}

    render(ui, View, title: "A much longer updated label")
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{type: :bounds, event: "rich-body-bounds", value: %{width: second}}}}

    assert second > first
    press(ui, "escape")
  end
end
