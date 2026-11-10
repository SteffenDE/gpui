defmodule GPUI.Test.Native.ViewportTest do
  use GPUI.Test, native: [size: {320, 160}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      GPUI.UI.button(%{id: "scaled-button", label: "Scaling", size: "sm", "phx-click": "clicked"})
      |> GPUI.Element.with_rem_size(assigns.size)
    end
  end

  test "native control rem geometry follows root size and resets on rerender", %{ui: ui} do
    render(ui, View, %{size: 16})
    settle(ui)
    initial = bounds(ui, "scaled-button").height
    render(ui, View, %{size: 20})
    settle(ui)
    assert_in_delta bounds(ui, "scaled-button").height, initial * 1.25, 0.1
    click(ui, "scaled-button")
    assert_receive {:gpui, ^ui, {:event, %{type: :click, event: "clicked"}}}
    render(ui, View, %{size: 16})
    settle(ui)
    assert_in_delta bounds(ui, "scaled-button").height, initial, 0.1
  end
end
