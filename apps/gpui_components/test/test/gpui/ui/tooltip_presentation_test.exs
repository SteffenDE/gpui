defmodule GPUI.UI.TooltipPresentationTest do
  use ExUnit.Case, async: true
  alias GPUI.Template.Component.Slot
  alias GPUI.UI.Overlay

  defp slots do
    %{
      id: "help",
      trigger: [%Slot{children: ["Trigger"]}],
      content: [%Slot{children: ["File history"]}]
    }
  end

  test "plain tooltips retain their native appearance and cursor placement" do
    tooltip = Overlay.tooltip(slots())
    assert tooltip.attrs[:appearance]
    refute tooltip.attrs[:keyboard]
    assert tooltip.attrs[:placement] == "cursor"
    assert tooltip.attrs[:delay] == 500.0
    assert tooltip.attrs[:collision_padding] == 4.0
    assert [%GPUI.Element{type: :ui_tooltip_trigger}] = tooltip.children
  end

  test "styled body and shortcut survive native payload conversion" do
    tooltip =
      slots()
      |> Map.merge(%{
        appearance: false,
        keyboard: true,
        placement: "top",
        offset: 4,
        collision_padding: 5,
        content: [%Slot{attrs: [class: "text-[12px]"], children: ["File history"]}],
        shortcut: [%Slot{attrs: [class: "pl-1.5", letter_spacing: 1.2], children: ["H"]}]
      })
      |> Overlay.tooltip()
      |> GPUI.Element.to_payload()

    assert tooltip.attrs.appearance == false
    assert tooltip.attrs.keyboard
    assert tooltip.attrs.placement == "top"
    assert tooltip.attrs.collision_padding == 5
    assert [_, body, shortcut] = tooltip.children
    assert body.type == :ui_tooltip_content
    assert body.attrs.text == "File history"
    assert shortcut.type == :ui_tooltip_shortcut
    assert shortcut.attrs.text == "H"
    assert shortcut.attrs.letter_spacing == 1.2
  end

  test "rich display content and edge alignment survive payload conversion" do
    icon = %GPUI.Element{type: :svg, attrs: [data: "<svg/>"]}

    label = %GPUI.Element{
      type: :text,
      attrs: [class: "text-[#abaeb4]"],
      children: ["Full context"]
    }

    tooltip =
      slots()
      |> Map.merge(%{
        placement: "top_start",
        appearance: false,
        content: [%Slot{children: [icon, label]}]
      })
      |> Overlay.tooltip()
      |> GPUI.Element.to_payload()

    assert tooltip.attrs.placement == "top_start"
    assert [_, body] = tooltip.children
    assert body.attrs.text == ""
    assert [vector, text] = body.children
    assert vector.type == :svg
    assert text.children == ["Full context"]
    assert text.attrs.style[:color] == [:rgb, 0xABAEB4]
  end

  test "shortcut letter spacing rejects nonnumeric values before transport" do
    assert_raise ArgumentError, fn ->
      slots()
      |> Map.put(:shortcut, [%Slot{attrs: [letter_spacing: "wide"], children: ["⌘⇧F"]}])
      |> Overlay.tooltip()
      |> GPUI.Element.to_payload()
    end
  end

  test "invalid placement, offset and repeated shortcuts are rejected" do
    for overrides <- [
          %{placement: "diagonal"},
          %{offset: -1},
          %{close_delay: -1},
          %{close_delay: nil},
          %{close_delay: 1.0e300},
          %{collision_padding: -1},
          %{collision_padding: "5"},
          %{collision_padding: 1.0e300},
          %{appearance: nil},
          %{keyboard: nil},
          %{shortcut: [%Slot{children: ["A"]}, %Slot{children: ["B"]}]}
        ] do
      assert_raise ArgumentError, fn -> slots() |> Map.merge(overrides) |> Overlay.tooltip() end
    end
  end
end
