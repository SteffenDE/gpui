defmodule GPUI.SystemNotificationsTest do
  use ExUnit.Case, async: false
  alias GPUI.SystemNotifications

  defmodule Backend do
    def compiled?, do: true

    def show_system_notification(tag, title, subtitle, body, sound) do
      send(self(), {:notification_queued, tag, title, subtitle, body, sound})
      {:ok, :ok}
    end

    def dismiss_system_notification(tag) do
      send(self(), {:notification_dismissed, tag})
      {:ok, :ok}
    end

    def subscribe_system_notifications do
      send(self(), :notification_subscribed)
      {:ok, :ok}
    end

    def system_notification_status, do: Process.get(:notification_status)

    def request_system_notifications do
      send(self(), :explicit_request)
      Process.get(:notification_request)
    end
  end

  setup do
    previous = Application.get_env(:gpui, :native_backend)
    Application.put_env(:gpui, :native_backend, Backend)

    on_exit(fn ->
      if previous,
        do: Application.put_env(:gpui, :native_backend, previous),
        else: Application.delete_env(:gpui, :native_backend)
    end)
  end

  test "reading each status never requests authorization" do
    for status <- [:authorized, :denied, :not_determined, :unsupported] do
      Process.put(:notification_status, {:ok, Atom.to_string(status)})
      assert SystemNotifications.status() == {:ok, status}
      refute_received :explicit_request
    end
  end

  test "only the explicit request API asks for permission and propagates errors" do
    Process.put(:notification_request, {:ok, "denied"})
    assert SystemNotifications.request() == {:ok, :denied}
    assert_received :explicit_request
    Process.put(:notification_request, {:error, "OS callback failed"})
    assert SystemNotifications.request() == {:error, "OS callback failed"}
  end

  test "delivery retains subtitle and sound without requesting permission" do
    assert :ok =
             SystemNotifications.show("thread",
               title: "Inbox",
               subtitle: "acme/repo · PR #42",
               body: "Activity",
               sound: true
             )

    assert_received {:notification_queued, "thread", "Inbox", "acme/repo · PR #42", "Activity",
                     true}

    assert :ok = SystemNotifications.dismiss("thread")
    assert_received {:notification_dismissed, "thread"}
    assert :ok = SystemNotifications.subscribe()
    assert_received :notification_subscribed
    refute_received :explicit_request
  end

  test "invalid or oversized notifications never reach the native boundary" do
    for {tag, options} <- [
          {"", []},
          {String.duplicate("x", 513), []},
          {"tag", [title: String.duplicate("x", 1025)]},
          {"tag", [subtitle: String.duplicate("x", 1025)]},
          {"tag", [body: String.duplicate("x", 16385)]},
          {"tag", [body: <<255>>]},
          {"tag", [sound: "yes"]},
          {"tag", [unknown: true]}
        ] do
      assert {:error, :invalid_notification} = SystemNotifications.show(tag, options)
    end

    assert {:error, :invalid_notification} = SystemNotifications.dismiss("")
    refute_received {:notification_queued, _, _, _, _, _}
    refute_received :explicit_request
  end
end
