defmodule GPUI.Test.Native.RichSelectionTest do
  use GPUI.Test, native: [size: {360, 240}]

  defmodule View do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col w-full h-full">
        <UI.rich_text id="code" label="Code" text={Map.get(assigns, :text, "Orchard 🌱 café")} runs={[]}
          selectable={assigns.selectable} phx-change="selected" class="w-[300px] h-8 text-[13px]" />
        <button id="outside" phx-click="outside" class="w-24 h-8">Outside</button>
      </div>
      """
    end
  end

  test "Select All reports Unicode text and Escape reports a cleared selection", %{ui: ui} do
    render(ui, View, selectable: true)
    settle(ui)
    click(ui, "code")
    press(ui, "cmd-a")
    assert_receive {:gpui, ^ui, {:event, %{event: "selected", value: "Orchard 🌱 café"}}}
    press(ui, :escape)
    assert_receive {:gpui, ^ui, {:event, %{event: "selected", value: ""}}}
  end

  test "pointer selection reports selected text and an outside click clears it", %{ui: ui} do
    render(ui, View, selectable: true)
    settle(ui)
    %{x: x, y: y} = bounds(ui, "code")
    drag(ui, {x + 1, y + 8}, {x + 50, y + 8})
    assert_receive {:gpui, ^ui, {:event, %{event: "selected", value: text}}} when text != ""
    assert String.starts_with?("Orchard 🌱 café", text)
    click(ui, "outside")
    assert_receive {:gpui, ^ui, {:event, %{event: "selected", value: ""}}}
  end

  test "replacing selected content reports a cleared selection", %{ui: ui} do
    render(ui, View, selectable: true)
    settle(ui)
    click(ui, "code")
    press(ui, "cmd-a")
    assert_receive {:gpui, ^ui, {:event, %{event: "selected", value: "Orchard 🌱 café"}}}
    render(ui, View, selectable: true, text: "Updated source")
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "selected", value: ""}}}
  end

  test "non-selectable text ignores Select All", %{ui: ui} do
    render(ui, View, selectable: false)
    settle(ui)
    click(ui, "code")
    press(ui, "cmd-a")
    refute_receive {:gpui, ^ui, {:event, %{event: "selected"}}}, 20
  end
end
