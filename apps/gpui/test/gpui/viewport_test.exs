defmodule GPUI.ViewportTest do
  use ExUnit.Case, async: true

  test "root rem size serializes as a validated viewport reference" do
    root = %GPUI.Element{type: :text, children: ["Scaled"]}
    tree = root |> GPUI.Element.with_rem_size(20) |> GPUI.Element.to_payload()
    assert tree.type == :viewport
    assert tree.attrs.rem_size == 20
    assert [%{type: :text, children: ["Scaled"]}] = tree.children
    assert_raise ArgumentError, fn -> GPUI.Element.with_rem_size(root, 0) end

    assert_raise ArgumentError, fn ->
      GPUI.Element.to_payload(%GPUI.Element{type: :viewport, attrs: [rem_size: -1]})
    end
  end
end
