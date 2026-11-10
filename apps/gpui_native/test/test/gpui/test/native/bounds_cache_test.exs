defmodule GPUI.Test.Native.BoundsCacheTest do
  use GPUI.Test, native: [size: {320, 240}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div class="relative w-full h-full">
        {Enum.map(assigns.first..(assigns.first + 299), &observer/1)}
      </div>
      """
    end

    defp observer(index) do
      assigns = %{index: index}

      ~GPUI"""
      <div id={"bounds-#{assigns.index}"} phx-bounds-change="observed" class="absolute w-1 h-1"
        style={[left: {:px, rem(assigns.index, 20) * 10.0}, top: {:px, rem(div(assigns.index, 20), 20) * 10.0}]} />
      """
    end
  end

  test "more than 256 observers and replacement views report bounds without repeating unchanged geometry",
       %{ui: ui} do
    render(ui, View, %{first: 0})
    settle(ui)

    for index <- 0..299 do
      id = "bounds-#{index}"
      assert_receive {:gpui, ^ui, {:event, %{event: "observed", value: %{id: ^id}}}}
    end

    render(ui, View, %{first: 0})
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "observed"}}}, 10
    render(ui, View, %{first: 300})
    settle(ui)

    for index <- 300..599 do
      id = "bounds-#{index}"
      assert_receive {:gpui, ^ui, {:event, %{event: "observed", value: %{id: ^id}}}}
    end

    render(ui, View, %{first: 300})
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "observed"}}}, 10
  end
end
