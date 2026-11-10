defmodule GPUI.Test.Native.MultilineHorizontalTest do
  use GPUI.Test, native: [size: {360, 300}]

  defmodule View do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col gap-3.5 w-full">
        <UI.rich_text id="first" label="First code" text={assigns.text} runs={[]}
          scroll_group="first" scroll_revision={assigns.revision} scrollbar={true}
          scrollbar_placement={Map.get(assigns, :placement, "gutter")} scroll_padding={0} line_height_snap="none"
          phx-scroll="first-scroll" style={[width: {:px, Map.get(assigns, :width, 200) * 1.0}]}
          class="min-w-0 px-3 py-2 text-[15px] leading-[22.3px] whitespace-nowrap" />
        <UI.rich_text id="second" label="Second code" text={assigns.text} runs={[]}
          scroll_group="second" phx-scroll="second-scroll" scrollbar={true} scrollbar_placement={Map.get(assigns, :placement, "gutter")} scroll_padding={0}
          class="w-[200px] min-w-0 text-[15px] leading-[22.3px] whitespace-nowrap" />
      </div>
      """
    end
  end

  for tabbed <- [false, true] do
    test "multiline #{if tabbed, do: "tabbed", else: "styled"} code measures its widest line and scrolls independently",
         %{ui: ui} do
      text =
        "short\n" <> if(unquote(tabbed), do: "🌱\t", else: "") <> String.duplicate("wide ", 100)

      render(ui, View, text: text, revision: 1)
      settle(ui)

      assert_receive {:gpui, ^ui,
                      {:event, %{event: "first-scroll", value: [+0.0, maximum, 200.0, _]}}}

      assert_receive {:gpui, ^ui, {:event, %{event: "second-scroll", value: [+0.0, _, _, _]}}}
      assert maximum > 1000
      assert_in_delta bounds(ui, "first").height, 2 * 22.3 + 16 + 10, 0.51
      scroll(ui, "first", delta: {-200, 0})
      settle(ui)
      assert_receive {:gpui, ^ui, {:event, %{event: "first-scroll", value: [200.0, _, _, _]}}}
      refute_receive {:gpui, ^ui, {:event, %{event: "second-scroll"}}}, 20

      render(ui, View, text: "short\nsmall", revision: 2)
      settle(ui)

      assert_receive {:gpui, ^ui,
                      {:event, %{event: "first-scroll", value: [+0.0, +0.0, 200.0, _]}}}

      assert_in_delta bounds(ui, "first").height, 2 * 22.3 + 16, 0.51
    end
  end

  for placement <- ["gutter", "outside"] do
    test "empty #{placement} content retains its padding without a phantom text line", %{ui: ui} do
      render(ui, View, text: "", revision: 1, placement: unquote(placement))
      settle(ui)
      assert_in_delta bounds(ui, "first").height, 16.0, 0.001
      assert_in_delta bounds(ui, "second").height, 0.0, 0.001
    end
  end

  test "outside scrollbars preserve automatic block height through viewport resizing", %{ui: ui} do
    text = "short\n" <> String.duplicate("w", 25)
    render(ui, View, text: text, revision: 1, placement: "outside")
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "first-scroll", value: [left, maximum, width, _]}}}

    assert left == 0 and maximum > 0 and width == 200
    height = bounds(ui, "first").height
    assert_in_delta height, 2 * 22.3 + 16, 0.51
    render(ui, View, text: text, revision: 1, placement: "outside", width: 340)
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "first-scroll", value: [left, maximum, width, _]}}}

    assert left == 0 and maximum == 0 and width == 340
    assert bounds(ui, "first").height == height
  end

  test "the outside thumb accepts dragging below the text element bounds", %{ui: ui} do
    render(ui, View,
      text: "short\n" <> String.duplicate("wide ", 100),
      revision: 1,
      placement: "outside"
    )

    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "first-scroll"}}}
    y = bounds(ui, "first").height + 5
    drag(ui, {8, y}, {80, y})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "first-scroll", value: [left, _, _, _]}}}
    assert left > 100
  end
end
