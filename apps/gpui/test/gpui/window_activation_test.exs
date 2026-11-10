defmodule GPUI.WindowActivationTest do
  use ExUnit.Case, async: true

  defmodule View do
    use GPUI.View
    def render(_), do: %GPUI.Element{type: :text, children: ["Activate"]}
  end

  defmodule App do
    use GPUI.Application
    def mount(_), do: {:ok, [%GPUI.WindowSpec{title: "Activate", root: {View, %{}}}]}
  end

  defmodule Display do
    def start_link(opts), do: Agent.start_link(fn -> opts[:test_owner] end)
    def sync(_, _), do: :ok
    def drain_events(_), do: {:ok, []}
    def inject_event(_, event), do: {:ok, event}

    def activate_window(display, id) do
      send(Agent.get(display, & &1), {:activated, id})
      if id == 1, do: :ok, else: {:error, :window_not_found}
    end
  end

  test "activation forwards the session window ID without changing the model" do
    runtime =
      start_supervised!(
        {GPUI.Runtime,
         app: App, display: Display, display_opts: [test_owner: self()], poll_interval: nil}
      )

    revision = GPUI.Runtime.info(runtime).revision
    assert :ok = GPUI.Runtime.activate_window(runtime, 1)
    assert_receive {:activated, 1}
    assert GPUI.Runtime.info(runtime).revision == revision
    assert {:error, :window_not_found} = GPUI.Runtime.activate_window(runtime, 99)
  end

  test "displays without an activation capability report unsupported" do
    runtime =
      start_supervised!({GPUI.Runtime, app: App, display: GPUI.Test.Display, poll_interval: nil})

    assert {:error, :activation_not_supported} = GPUI.Runtime.activate_window(runtime, 1)
  end

  test "invalid IDs never call the runtime" do
    for id <- [0, -1, nil, "1"] do
      assert {:error, :invalid_window_id} = GPUI.Runtime.activate_window(:missing_runtime, id)
    end
  end
end
