defmodule GPUI.WindowGeometryTest do
  use GPUI.Test, async: true

  defmodule View do
    use GPUI.View
    def render(_), do: %GPUI.Element{type: :div}

    def handle_window_event(:geometry, %{value: frame}, assigns),
      do: {:noreply, Map.put(assigns, :frame, frame)}
  end

  defmodule App do
    use GPUI.Application

    def mount(_) do
      {:ok,
       [
         window "Geometry" do
           size(800, 600)
           restore_bounds({100, 200, 900, 700})
           cascade_offset(36)
           geometry_events(true)
           center_in_work_area(true)
           root(View)
         end
       ]}
    end
  end

  test "window placement and geometry opt-in survive serialization and lifecycle dispatch" do
    runtime = start_runtime!(App)
    window = hd(GPUI.Runtime.snapshot!(runtime).windows)
    assert window.restore_bounds == [100.0, 200.0, 900.0, 700.0]
    assert window.cascade_offset == 36
    assert window.center_in_work_area
    assert :geometry in window.lifecycle
    frame = %{bounds: [-100.0, 200.0, 900.0, 700.0], initial: false}
    dispatch(runtime, %{type: :window_geometry, window_id: 1, value: frame})
    assert assigns(runtime).frame == frame
    refute :window_geometry in GPUI.Event.injectable_types()
  end

  test "rejects malformed placement and geometry values" do
    for bounds <- [{0, 0, 0, 400}, {0, 0, 900, -1}, {1, 2}, {1.0e20, 2, 900, 700}] do
      assert_raise ArgumentError, fn -> GPUI.WindowSpec.new("Invalid", restore_bounds: bounds) end
    end

    assert_raise ArgumentError, fn -> GPUI.WindowSpec.new("Invalid", cascade_offset: -1) end

    assert_raise ArgumentError, fn ->
      GPUI.WindowSpec.new("Invalid", center_in_work_area: :yes)
    end

    assert_raise ArgumentError, fn -> GPUI.WindowSpec.new("Invalid", geometry_events: :yes) end

    for value <- [
          %{},
          %{bounds: [0, 0, 0, 700], initial: true},
          %{bounds: [0, 0, 900, 700], initial: :yes}
        ] do
      assert {:error, {:invalid_event, :value}} =
               GPUI.Event.normalize(%{type: :window_geometry, window_id: 1, value: value})
    end
  end
end
