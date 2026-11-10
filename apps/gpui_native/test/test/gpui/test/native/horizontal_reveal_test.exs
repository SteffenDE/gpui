defmodule GPUI.Test.Native.HorizontalRevealTest do
  use GPUI.Test, native: [size: {360, 240}]

  defmodule View do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col w-[300px] h-full">
        <div id="long-row" class="flex h-8 w-full">
          <UI.rich_text id="long-code" label="Long code" text={"🌱" <> String.duplicate("x", 300) <> "target" <> String.duplicate("y", 100)} runs={[]}
            scroll_group="file" scroll_pane="long" phx-scroll="long_scroll"
            reveal_start={elem(assigns.range, 0)} reveal_end={elem(assigns.range, 1)} reveal_request={assigns.request}
            class="w-full h-8 text-[13px] whitespace-nowrap" />
        </div>
        <UI.rich_text id="short-code" label="Short code" text="short" runs={[]}
          scroll_group="file" scroll_pane="short" phx-scroll="short_scroll" class="w-full h-8 text-[13px] whitespace-nowrap" />
      </div>
      """
    end
  end

  test "shaped UTF-16 reveal centers distant text and only repeats for a new request", %{ui: ui} do
    render(ui, View, range: {302, 308}, request: 1)
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "long_scroll", value: [left, _, _, _]}}}
                   when left > 1000

    flush(ui)
    scroll(ui, "long-row", delta: {10_000, 0})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "long_scroll", value: [+0.0, _, _, _]}}}
    flush(ui)
    render(ui, View, range: {302, 308}, request: 1)
    settle(ui)

    refute_receive {:gpui, ^ui, {:event, %{event: "long_scroll"}}}, 20

    render(ui, View, range: {302, 308}, request: 2)
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "long_scroll", value: [left, _, _, _]}}}
                   when left > 1000

    refute_receive {:gpui, ^ui, {:event, %{event: "short_scroll"}}}, 20
  end

  test "fully visible ranges preserve a user scroll position", %{ui: ui} do
    render(ui, View, range: {12, 14}, request: 1)
    settle(ui)
    flush(ui)
    scroll(ui, "long-row", delta: {-40, 0})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "long_scroll", value: [40.0, _, _, _]}}}
    flush(ui)
    render(ui, View, range: {12, 14}, request: 2)
    settle(ui)

    refute_receive {:gpui, ^ui, {:event, %{event: "long_scroll"}}}, 20
  end

  defp flush(ui) do
    receive do
      {:gpui, ^ui, _} -> flush(ui)
    after
      0 -> :ok
    end
  end
end
