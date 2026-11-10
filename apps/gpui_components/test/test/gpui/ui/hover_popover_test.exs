defmodule GPUI.UI.HoverPopoverTest do
  use ExUnit.Case, async: true
  alias GPUI.Template.Component.Slot
  alias GPUI.UI.Overlay

  test "hover previews retain explicit timing and focus policy" do
    assigns = %{
      id: "preview",
      label: "Preview",
      trigger: [%Slot{children: ["Hover"]}],
      content: [%Slot{children: ["Details"]}]
    }

    ordinary = Overlay.popover(assigns)
    refute ordinary.attrs[:open_on_hover]
    assert ordinary.attrs[:focus_on_open]
    refute ordinary.attrs[:trigger_passthrough]

    hover =
      Overlay.popover(
        Map.merge(assigns, %{
          open_on_hover: true,
          focus_on_open: false,
          trigger_passthrough: true,
          hover_delay: 50,
          close_delay: 200
        })
      )

    assert hover.attrs[:open_on_hover]
    refute hover.attrs[:focus_on_open]
    assert hover.attrs[:trigger_passthrough]
    assert hover.attrs[:hover_delay] == 50
    assert hover.attrs[:close_delay] == 200
    assert GPUI.Element.to_payload(hover).type == :ui_popover
  end

  test "virtual anchors require a complete finite rectangle" do
    assigns = %{
      id: "virtual",
      label: "Mention",
      trigger: [%Slot{children: []}],
      content: [%Slot{children: ["Profile"]}]
    }

    rectangle = %{
      anchor_x: 50.5,
      anchor_y: 70,
      anchor_width: 42,
      anchor_height: 20,
      anchor_hovered: true,
      parent_popover: "parent-preview",
      open_on_hover: true,
      focus_on_open: false
    }

    node = Overlay.popover(Map.merge(assigns, rectangle))
    assert node.attrs[:anchor_x] == 50.5
    assert node.attrs[:anchor_hovered]
    assert node.attrs[:parent_popover] == "parent-preview"

    for invalid <- [
          Map.delete(rectangle, :anchor_y),
          %{rectangle | anchor_width: -1},
          %{rectangle | anchor_x: 1_000_001},
          %{rectangle | anchor_hovered: 1},
          %{rectangle | parent_popover: ""},
          %{rectangle | parent_popover: 1}
        ] do
      assert_raise ArgumentError, fn -> Overlay.popover(Map.merge(assigns, invalid)) end
    end
  end

  test "ordinary containers can request focus for a keyboard region" do
    node = %GPUI.Element{
      type: :div,
      attrs: [id: "commits-region", focus_request: 1],
      children: []
    }

    assert GPUI.Element.to_payload(node).attrs[:focus_request] == 1
  end

  test "popover timing is bounded and activation policies are booleans" do
    assigns = %{
      id: "preview",
      label: "Preview",
      trigger: [%Slot{children: ["Hover"]}],
      content: [%Slot{children: ["Details"]}]
    }

    for option <- [
          %{hover_delay: -1},
          %{close_delay: 60_001},
          %{open_on_hover: "yes"},
          %{focus_on_open: 1},
          %{trigger_passthrough: "true"}
        ] do
      assert_raise ArgumentError, fn -> Overlay.popover(Map.merge(assigns, option)) end
    end
  end
end
