defmodule GPUI.UI.RichRevealTest do
  use ExUnit.Case, async: true
  alias GPUI.UI

  test "reveal offsets use paired, valid UTF-16 boundaries in a horizontal group" do
    options = %{id: "code", label: "Code", text: "🌱x", runs: [], scroll_group: "file"}

    assert UI.rich_text(Map.merge(options, %{reveal_start: 0, reveal_end: 2})).type ==
             :ui_rich_text

    for extra <- [
          %{reveal_start: 1, reveal_end: 2},
          %{reveal_start: 2, reveal_end: 4},
          %{reveal_start: 2},
          %{reveal_start: 2, reveal_end: 0},
          %{reveal_start: 0, reveal_end: 2, scroll_group: nil}
        ] do
      assert_raise ArgumentError, fn -> UI.rich_text(Map.merge(options, extra)) end
    end
  end
end
