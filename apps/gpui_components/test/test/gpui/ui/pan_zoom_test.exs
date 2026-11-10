defmodule GPUI.UI.PanZoomTest do
  use ExUnit.Case, async: true
  alias GPUI.{Element, UI}

  defp options(extra \\ %{}),
    do:
      Map.merge(
        %{
          id: "canvas",
          label: "Image",
          content_width: 160,
          content_height: 80,
          "phx-change": "transform",
          children: [%Element{type: :div}]
        },
        extra
      )

  test "builds a controlled viewport with a single content element" do
    view = UI.pan_zoom(options())
    assert view.type == :ui_pan_zoom
    assert Keyword.get(view.attrs, :min_scale) == 0.25
    assert Keyword.get(view.attrs, :max_scale) == 10.0
    assert Keyword.get(view.attrs, :reset_request) == 0
    assert length(view.children) == 1
  end

  test "rejects invalid geometry, scale ranges, children and missing controlled events" do
    for extra <- [
          %{content_width: 0},
          %{scale: 0},
          %{min_scale: 2, scale: 1},
          %{max_scale: 0.5},
          %{children: []},
          %{children: [%Element{type: :div}, %Element{type: :div}]}
        ] do
      assert_raise ArgumentError, fn -> UI.pan_zoom(options(extra)) end
    end

    assert_raise ArgumentError, fn -> UI.pan_zoom(Map.delete(options(), :"phx-change")) end
    assert_raise ArgumentError, fn -> UI.pan_zoom(Map.delete(options(), :content_width)) end
  end
end
