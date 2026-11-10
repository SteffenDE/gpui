defmodule GPUI.EventTest do
  use ExUnit.Case, async: true

  alias GPUI.Event

  test "exposes one native injection subset from the canonical event contract" do
    assert Event.injectable_types() == [
             :click,
             :command,
             :change,
             :open,
             :release,
             :search,
             :submit,
             :keydown,
             :keyup,
             :window_close_request,
             :window_focus,
             :window_blur,
             :window_closed
           ]
  end

  test "click modifiers are optional and validated without changing plain click events" do
    event = %{type: :click, window_id: 1, event: "open"}
    assert {:ok, ^event} = Event.normalize(event)
    modifiers = %{control: false, alt: false, shift: true, platform: true}
    modified = Map.put(event, :modifiers, modifiers)
    assert {:ok, ^modified} = Event.normalize(modified)

    for invalid <- [nil, %{}, %{modifiers | shift: "yes"}] do
      assert {:error, {:invalid_event, :modifiers}} =
               Event.normalize(Map.put(event, :modifiers, invalid))
    end
  end

  test "requires an explicit known event type" do
    assert {:error, {:invalid_event, :type}} =
             Event.normalize(%{window_id: 1, event: "save"})

    assert {:error, {:unsupported_event_type, :mystery}} =
             Event.normalize(%{type: :mystery, window_id: 1})

    assert {:error, {:invalid_event, :type}} =
             Event.normalize(%{type: "click", window_id: 1, event: "save"})
  end

  test "requires a positive window id and bounded non-empty routed event name" do
    assert {:error, {:invalid_event, :window_id}} =
             Event.normalize(%{type: :click, event: "save"})

    assert {:error, {:invalid_event, :window_id}} =
             Event.normalize(%{type: :click, window_id: 0, event: "save"})

    assert {:error, {:invalid_event, :event}} =
             Event.normalize(%{type: :click, window_id: 1, event: ""})

    assert {:error, {:invalid_event, :event}} =
             Event.normalize(%{type: :click, window_id: 1, event: :save})

    assert {:error, {:invalid_event, :event}} =
             Event.normalize(%{type: :click, window_id: 1, event: :binary.copy("x", 513)})
  end

  test "accepts lifecycle events without application event names" do
    assert {:ok, %{type: :window_focus, window_id: 1}} =
             Event.normalize(%{type: :window_focus, window_id: 1})

    assert {:error, {:invalid_event, :event}} =
             Event.normalize(%{type: :window_focus, window_id: 1, event: "focus"})
  end

  test "validates representative event value shapes" do
    assert {:ok, %{type: :click}} =
             Event.normalize(%{type: :click, window_id: 1, event: "save"})

    assert {:error, {:invalid_event, :value}} =
             Event.normalize(%{type: :click, window_id: 1, event: "save", value: true})

    assert {:ok, %{value: false}} =
             Event.normalize(%{
               type: :change,
               window_id: 1,
               event: "enabled",
               value: false
             })

    assert {:error, {:invalid_event, :value}} =
             Event.normalize(%{type: :change, window_id: 1, event: "enabled", value: %{}})

    assert {:ok, %{value: %{first: 2, last: 5}}} =
             Event.normalize(%{
               type: :range,
               window_id: 1,
               event: "visible",
               value: %{first: 2, last: 5}
             })

    assert {:error, {:invalid_event, :value}} =
             Event.normalize(%{
               type: :range,
               window_id: 1,
               event: "visible",
               value: %{first: 5, last: 2}
             })
  end

  test "rich link hover carries finite span geometry and an explicit exit" do
    value = %{
      id: "message:0",
      link: "@mira",
      hovered: true,
      x: 12.5,
      y: 20,
      width: 42,
      height: 20,
      coordinate_space: "window_native_pixels"
    }

    event = %{type: :link_hover, window_id: 1, event: "mention", value: value}
    assert {:ok, ^event} = Event.normalize(event)
    assert {:ok, _} = Event.normalize(%{event | value: %{value | hovered: false}})

    for invalid <- [
          Map.delete(value, :id),
          Map.delete(value, :x),
          %{value | width: -1},
          %{value | hovered: "true"}
        ] do
      assert {:error, _} = Event.normalize(%{event | value: invalid})
    end
  end

  test "pointer observations require hover and all modifier booleans" do
    value = %{hovered: true, control: false, alt: false, shift: true, platform: true}
    event = %{type: :pointer, window_id: 1, event: "header", value: value}
    assert {:ok, ^event} = GPUI.Event.normalize(event)
    assert {:error, _} = GPUI.Event.normalize(%{event | value: Map.delete(value, :hovered)})
    assert {:error, _} = GPUI.Event.normalize(%{event | value: %{value | shift: "true"}})
  end
end
