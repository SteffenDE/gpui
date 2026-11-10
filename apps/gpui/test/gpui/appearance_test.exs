defmodule GPUI.AppearanceTest do
  use GPUI.Test, async: true

  defmodule View do
    use GPUI.View
    def render(assigns), do: %GPUI.Element{type: :text, children: [to_string(assigns.mode)]}

    def handle_window_event(:appearance, %{value: mode}, assigns),
      do: {:noreply, %{assigns | mode: mode}}
  end

  defmodule App do
    use GPUI.Application

    def mount(_) do
      {:ok,
       [
         window "Appearance" do
           appearance_events(true)
           root(View, mode: :light)
         end
       ]}
    end
  end

  test "appearance subscriptions route resolved renderer events to the window view" do
    runtime = start_runtime!(App)
    assert :appearance in hd(GPUI.Runtime.snapshot!(runtime).windows).lifecycle
    dispatch(runtime, %{type: :window_appearance, window_id: 1, value: :dark})
    assert assigns(runtime).mode == :dark

    assert {:error, {:invalid_event, :value}} =
             GPUI.Event.normalize(%{type: :window_appearance, window_id: 1, value: :system})

    refute :window_appearance in GPUI.Event.injectable_types()
  end

  test "runtime changes the display theme including system appearance" do
    runtime = start_runtime!(App, display_opts: [name: __MODULE__.Display])

    for mode <- [:light, :dark, :system] do
      assert :ok = GPUI.Runtime.set_theme(runtime, mode)
      assert GPUI.Test.Display.theme(__MODULE__.Display) == mode
    end
  end

  test "appearance notifications are opt-in" do
    spec = GPUI.WindowSpec.new("Default", root: {View, %{mode: :light}})
    refute spec.appearance_events

    assert_raise ArgumentError, fn ->
      GPUI.WindowSpec.new("Invalid", root: View, appearance_events: :yes)
    end
  end
end
