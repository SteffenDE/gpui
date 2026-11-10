defmodule GPUI.Test.Native.SvgTest do
  use GPUI.Test, native: [size: {160, 80}]

  defmodule View do
    use GPUI.View

    def render(_assigns) do
      ~GPUI"""
      <div class="flex items-center w-full h-full">
        <button id="vector-action" phx-click="vector_clicked" class="flex items-center justify-center w-8 h-8 text-[#79b8ff] bg-[#101317] hover:text-[#d7dbe1] hover:bg-[#212428]">
          <svg data="<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path fill='currentColor' d='M2 2h12v12H2z'/></svg>" class="w-4 h-4 shrink-0" />
        </button>
        <div id="after-vector" class="w-6 h-6" />
      </div>
      """
    end
  end

  test "inline vectors preserve control layout and pointer activation", %{ui: ui} do
    render(ui, View, %{})
    assert %{width: 32.0, height: 32.0} = bounds(ui, "vector-action")
    assert %{x: 32.0, width: 24.0} = bounds(ui, "after-vector")
    click(ui, "vector-action")
    assert_receive {:gpui, ^ui, {:event, %{type: :click, event: "vector_clicked"}}}
  end
end
