defmodule GPUI.UI.DialogPositionTest do
  use ExUnit.Case, async: true
  alias GPUI.Template.Component.Slot
  alias GPUI.UI.Overlay

  test "dialogs preserve a bounded viewport-relative top position" do
    dialog =
      Overlay.dialog(%{
        id: "palette",
        title: "Go to file",
        margin_top_ratio: 0.2,
        content: [%Slot{children: ["Search"]}]
      })

    assert dialog.attrs[:margin_top_ratio] == 0.2
    assert GPUI.Element.to_payload(dialog).attrs[:margin_top_ratio] == 0.2

    for ratio <- [-0.1, 1.1, "20%"] do
      assert_raise ArgumentError, fn ->
        Overlay.dialog(%{
          id: "palette",
          title: "Search",
          margin_top_ratio: ratio,
          content: [%Slot{children: ["Search"]}]
        })
      end
    end

    assert_raise ArgumentError, ~r/not both/, fn ->
      Overlay.dialog(%{
        id: "palette",
        title: "Search",
        margin_top: 120,
        margin_top_ratio: 0.2,
        content: [%Slot{children: ["Search"]}]
      })
    end
  end

  test "confirmation pointer dismissal is independent from keyboard dismissal" do
    dialog =
      Overlay.dialog(%{
        id: "confirmation",
        title: "Remove repository?",
        pointer_dismiss: false,
        content: [%Slot{children: ["Cancel"]}]
      })
      |> GPUI.Element.to_payload()

    assert dialog.attrs.pointer_dismiss == false
    assert dialog.attrs.keyboard == true
    assert dialog.attrs.closable == true
  end
end
