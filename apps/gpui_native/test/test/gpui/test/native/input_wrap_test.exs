defmodule GPUI.Test.Native.InputWrapTest do
  use GPUI.Test, native: [size: {320, 200}]

  defmodule View do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      ~GPUI"""
      <div id="wrap-container" class="flex flex-col w-[240px]">
        <UI.input id="wrapped" label="Comment" multiline={true} submit_on_enter={false}
          appearance={false} value={assigns.text} phx-change="text"
          style={[font_family: :".ZedMono"]} class="w-full p-0 text-[16px] leading-[20px]" />
      </div>
      """
    end
  end

  test "hyphenated text uses available columns before wrapping", %{ui: ui} do
    for separator <- ["-", "\u2010"] do
      text = "prefix " <> Enum.join(["aaaa", "bbbb", "cccc", "dddd", "eeee", "ff"], separator)
      render(ui, View, text: text)
      settle(ui)
      assert bounds(ui, "wrap-container").height == 40.0
    end
  end

  test "underscores and nonbreaking hyphens retain unbroken runs", %{ui: ui} do
    for separator <- ["_", "\u2011"] do
      text = "prefix " <> Enum.join(["aaaa", "bbbb", "cccc", "dddd", "eeee", "ff"], separator)
      render(ui, View, text: text)
      settle(ui)
      assert bounds(ui, "wrap-container").height == 60.0
    end
  end

  test "path separators and embedded opening brackets keep their words together", %{ui: ui} do
    for separator <- ["/", "(", "["] do
      text = "prefix " <> Enum.join(["aaaa", "bbbb", "cccc", "dddd", "eeee", "ff"], separator)
      render(ui, View, text: text)
      settle(ui)
      assert bounds(ui, "wrap-container").height == 60.0
    end
  end

  defmodule TextView do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      ~GPUI"""
      <UI.rich_text id="wrapped-text" text={assigns.text} runs={[]} label="Wrapped punctuation"
        hit_test_x={0} hit_test_y={28} hit_test_request={assigns.request} phx-hit-test="row-start"
        style={[font_family: :".ZedMono",width: {:px,assigns.width},font_size: {:px,assigns.size}]}
        class="leading-[20px] whitespace-normal" />
      """
    end
  end

  test "visual row starts match WebKit URL and query punctuation", %{ui: ui} do
    cases = [
      {"prefix aaaa/bbbb/cccc/dddd/eeee/ff", 240.0, 16.0, 7},
      {"prefix aaaa?bbbb?cccc?dddd?eeee?ff", 240.0, 16.0, 22},
      {"prefix aaaa(bbbb(cccc(dddd(eeee(ff", 240.0, 16.0, 7},
      {"prefix aaaa[bbbb[cccc[dddd[eeee[ff", 240.0, 16.0, 7},
      {"prefix aaaa—bbbb—cccc—dddd—eeee—ff", 240.0, 16.0, 22},
      {"prefix aaaa–bbbb–cccc–dddd–eeee–ff", 240.0, 16.0, 22},
      {"![shot.png](https://github.com/user-attachments/assets/aaaaaaaa-bbbb-cccc-dddd-000000000101)",
       458.0, 13.0, 36}
    ]

    for {{text, width, size, column}, request} <- Enum.with_index(cases, 1) do
      render(ui, TextView, text: text, width: width, size: size, request: request)
      settle(ui)
      assert_receive {:gpui, ^ui, {:event, %{event: "row-start", value: [line, actual_column]}}}
      assert line == 0 and actual_column == column
    end
  end
end
