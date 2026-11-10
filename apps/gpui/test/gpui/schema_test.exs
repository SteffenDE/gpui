defmodule GPUI.SchemaTest do
  use ExUnit.Case, async: true

  test "defines the neutral core schema independently" do
    assert GPUI.Schema.tags() == [
             :div,
             :button,
             :layer,
             :span,
             :scroll,
             :list,
             :item,
             :text_surface,
             :text_input,
             :svg,
             :img,
             :text,
             :ui_terminal,
             :ui_terminal_row,
             :ui_terminal_cell,
             :ui_edge_fade,
             :ui_frost,
             :ui_paint
           ]

    assert :text_surface in GPUI.Schema.identified_tags()
    assert GPUI.Schema.component!(:text_surface).required_events == []
    assert Enum.map(GPUI.Schema.stateful_components(), & &1.kind) == [:text_surface]
    assert :raster in GPUI.Schema.resources()
    assert :border_radius in GPUI.Schema.styles()
    assert :font_weight in GPUI.Schema.styles()
    assert :font_family in GPUI.Schema.styles()
  end

  test "retains neutral defaults and validation" do
    assert %{max_lines: 8, min_lines: 1} = GPUI.Schema.defaults(:text_surface)

    assert_raise ArgumentError, ~r/unknown GPUI component/, fn ->
      GPUI.Schema.component!(:unknown)
    end
  end

  test "repeating motion validates its destination and preserves defaults" do
    defaults = GPUI.Schema.defaults(:div)
    assert defaults.motion_to_opacity == 1.0
    assert defaults.motion_to_x == 0.0
    assert defaults.motion_repeat == false

    assert %{motion_to_x: 80} =
             GPUI.Schema.validate_component_assigns!(
               %{id: "motion", motion_request: 1, motion_repeat: true, motion_to_x: 80},
               :div
             )

    assert_raise ArgumentError, fn ->
      GPUI.Schema.validate_component_assigns!(%{motion_to_opacity: 2.0}, :div)
    end

    assert_raise ArgumentError, fn ->
      GPUI.Schema.validate_component_assigns!(%{motion_to_x: 1_000_001}, :div)
    end

    assert GPUI.Schema.defaults(:svg).spin_duration == 0
    refute :svg in GPUI.Schema.identified_tags()

    assert_raise ArgumentError, fn ->
      GPUI.Schema.validate_component_assigns!(%{data: "<svg />", spin_duration: 1000}, :svg)
    end

    assert GPUI.Schema.validate_component_assigns!(
             %{data: "<svg />", id: "spinner", spin_duration: 1000},
             :svg
           )
  end
end
