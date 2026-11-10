defmodule GPUI.Native.BackendTest do
  use ExUnit.Case, async: true

  @moduletag :native

  test "generated lifecycle boundary starts and stops a native runtime" do
    assert GPUI.Native.Backend.available?()
    assert {:ok, host} = GPUI.Native.Backend.host_info()
    assert host in [:headless, :vanilla, :gpui_component]
    assert {:ok, runtime} = GPUI.Native.Backend.start_runtime()
    assert {:ok, :ok} = GPUI.Native.Backend.stop_runtime(runtime)
  end

  test "native events cross the boundary as structured maps" do
    assert {:ok, runtime} = GPUI.Native.Backend.start_runtime()

    assert {:ok, :ok} =
             GPUI.Native.Backend.inject_event(runtime, %{
               type: :click,
               window_id: 7,
               event: "save"
             })

    assert {:ok, [%{type: :click, window_id: 7, event: "save"}]} =
             GPUI.Native.Backend.drain_events(runtime)

    assert {:ok, :ok} =
             GPUI.Native.Backend.inject_event(runtime, %{
               type: :command,
               window_id: 7,
               event: "save"
             })

    assert {:ok, [%{type: :command, window_id: 7, event: "save"}]} =
             GPUI.Native.Backend.drain_events(runtime)

    assert {:ok, :ok} =
             GPUI.Native.Backend.inject_event(runtime, %{type: :window_closed, window_id: 7})

    assert {:ok, [%{type: :window_closed, window_id: 7}]} =
             GPUI.Native.Backend.drain_events(runtime)

    assert {:ok, :ok} =
             GPUI.Native.Backend.inject_event(runtime, %{
               type: :change,
               window_id: 7,
               event: "language_changed",
               value: nil
             })

    assert {:ok, [%{type: :change, window_id: 7, event: "language_changed", value: nil}]} =
             GPUI.Native.Backend.drain_events(runtime)

    assert {:ok, :ok} =
             GPUI.Native.Backend.inject_event(runtime, %{
               type: :keydown,
               window_id: 7,
               event: "key_pressed",
               value: "enter"
             })

    assert {:ok, [%{type: :keydown, window_id: 7, event: "key_pressed", value: "enter"}]} =
             GPUI.Native.Backend.drain_events(runtime)
  end

  test "modified clicks round-trip through the native event boundary" do
    assert {:ok, runtime} = GPUI.Native.Backend.start_runtime()

    event = %{
      type: :click,
      window_id: 7,
      event: "open",
      modifiers: %{control: false, alt: false, shift: true, platform: true}
    }

    assert {:ok, :ok} = GPUI.Native.Backend.inject_event(runtime, event)
    assert {:ok, [^event]} = GPUI.Native.Backend.drain_events(runtime)
    assert {:ok, :ok} = GPUI.Native.Backend.stop_runtime(runtime)
  end

  test "rejects malformed font configuration before starting a native runtime" do
    Process.flag(:trap_exit, true)

    for fonts <- [nil, "font.ttf", [123], [""], List.duplicate("font", 65)] do
      assert {:error, :invalid_fonts} = GPUI.Display.Native.start_link(fonts: fonts)
    end
  end

  test "rejects unsupported native component themes" do
    Process.flag(:trap_exit, true)

    assert {:error, {:invalid_theme, :sepia}} =
             GPUI.Display.Native.start_link(theme: :sepia)
  end

  test "keeps low-level native lifecycle operations behind the backend adapter" do
    Code.ensure_loaded!(GPUI.Native)
    Code.ensure_loaded!(GPUI.Native.Backend)

    assert function_exported?(GPUI.Native, :available?, 0)
    refute function_exported?(GPUI.Native, :start_runtime, 0)

    assert function_exported?(GPUI.Native.Backend, :decode_image, 1)
    assert function_exported?(GPUI.Native.Backend, :host_info, 0)
    assert function_exported?(GPUI.Native.Backend, :text_buffer_new, 3)
    assert function_exported?(GPUI.Native.Backend, :text_buffer_snapshot, 1)
    assert function_exported?(GPUI.Native.Backend, :text_buffer_transact, 2)
    assert function_exported?(GPUI.Native.Backend, :text_buffer_undo, 2)
    assert function_exported?(GPUI.Native.Backend, :text_buffer_redo, 2)
    assert function_exported?(GPUI.Native.Backend, :open_window, 2)
    assert function_exported?(GPUI.Native.Backend, :update_window, 3)
    assert function_exported?(GPUI.Native.Backend, :close_window, 2)
    assert function_exported?(GPUI.Native.Backend, :await_frame, 3)
    assert function_exported?(GPUI.Native.Backend, :frame_token, 2)
    assert function_exported?(GPUI.Native.Backend, :await_frame_after, 4)
    assert function_exported?(GPUI.Native.Backend, :stop_runtime, 1)
    assert function_exported?(GPUI.Native.Backend, :set_theme, 2)
    assert function_exported?(GPUI.Native.Backend, :load_font, 2)
  end
end
