defmodule GPUI.Test.Native.DecodeDepthTest do
  use GPUI.Test, native: [size: {240, 120}]

  defmodule View do
    use GPUI.View

    def render(assigns) do
      marker = %GPUI.Element{type: :div, attrs: [id: "deep-marker", class: "w-4 h-4"]}

      Enum.reduce(1..assigns.depth, marker, fn _, child ->
        %GPUI.Element{type: :div, attrs: [class: "w-full h-full"], children: [child]}
      end)
    end
  end

  test "deep native trees decode without exhausting the BEAM scheduler stack", %{ui: ui} do
    render(ui, View, depth: 64)
    assert %{width: 16.0, height: 16.0} = bounds(ui, "deep-marker")
  end

  test "excessive depth fails without killing the native session", %{ui: ui} do
    assert_raise GPUI.Test.NativeError, fn -> render(ui, View, depth: 129) end
    render(ui, View, depth: 4)
    assert %{width: 16.0} = bounds(ui, "deep-marker")
  end
end
