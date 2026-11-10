defmodule GPUI.Test.Native.ClipboardRequestTest do
  use GPUI.Test, native: [size: {300, 100}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div id="copy-root" clipboard_text={assigns.text} clipboard_write_request={assigns.request} phx-clipboard-write="copied" class="flex w-full h-full"><text_input id="draft" value="" focus_request={1} phx-change="typed" class="h-8 w-40" /></div>
      """
    end
  end

  test "requests acknowledge once, retain focus, and require a new positive token", %{ui: ui} do
    render(ui, View, %{request: 1, text: "diff 🌱"})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{type: :clipboard_write, event: "copied"}}}
    render(ui, View, %{request: 1, text: "changed"})
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "copied"}}}
    type(ui, "c")
    assert_receive {:gpui, ^ui, {:event, %{event: "typed", value: "c"}}}
    render(ui, View, %{request: 0, text: "changed"})
    settle(ui)
    render(ui, View, %{request: 1, text: "changed"})
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "copied"}}}
    render(ui, View, %{request: 2, text: "changed"})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "copied"}}}
  end
end
