defmodule GPUI.Text.RichRunTest do
  use ExUnit.Case, async: true
  alias GPUI.Text.{Position, Range, RichRun}

  test "font family alone is a shaping fact and survives payload serialization" do
    run =
      RichRun.new(Range.new(Position.new(0, 0), Position.new(0, 3)), font_family: "Example Mono")

    element = %GPUI.Element{type: :ui_rich_text, attrs: [runs: [run]], children: []}

    assert [%{font_family: "Example Mono", font_weight: nil}] =
             GPUI.Element.to_payload(element).attrs.runs
  end

  test "font families are bounded UTF-8 names" do
    range = Range.caret(Position.new(0, 0))

    for family <- ["", :monospace, 4, <<255>>, "bad" <> <<0>>, String.duplicate("x", 257)] do
      assert_raise ArgumentError, ~r/font_family/, fn ->
        RichRun.new(range, font_family: family)
      end
    end

    assert %RichRun{font_family: "庭 Mono"} = RichRun.new(range, font_family: "庭 Mono")
  end

  test "background opacity is optional, bounded and retained in the native payload" do
    range = Range.new(Position.new(0, 0), Position.new(0, 3))

    for alpha <- [0, 0.15, 0.2, 1] do
      run = RichRun.new(range, background: 0xF06E64, background_opacity: alpha)
      element = %GPUI.Element{type: :ui_rich_text, attrs: [runs: [run]], children: []}

      assert [%{background: 0xF06E64, background_opacity: ^alpha}] =
               GPUI.Element.to_payload(element).attrs.runs
    end

    for alpha <- [-0.1, 1.1, "0.2", true] do
      assert_raise ArgumentError, ~r/background_opacity/, fn ->
        RichRun.new(range, background: 0x123456, background_opacity: alpha)
      end
    end

    assert_raise ArgumentError, ~r/requires background/, fn ->
      RichRun.new(range, color: 0xFFFFFF, background_opacity: 0.2)
    end
  end
end
