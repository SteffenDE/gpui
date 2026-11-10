defmodule GPUI.SystemNotifications do
  @moduledoc """
  Process-wide operating-system notification authorization, delivery, and activation.

  `status/0` never prompts. `request/0` must be called in response to an explicit
  user action. Both wait for the OS callback; call from a supervised task rather
  than a view handler. A macOS process outside an app bundle returns
  `:unsupported` without touching UserNotifications, which would abort the process.
  Other platforms currently return `:unsupported` for permission queries.
  """
  @type status :: :authorized | :denied | :not_determined | :unsupported
  @spec status() :: {:ok, status()} | {:error, term()}
  def status, do: normalize(GPUI.Native.Backend.system_notification_status())

  @spec request() :: {:ok, status()} | {:error, term()}
  def request, do: normalize(GPUI.Native.Backend.request_system_notifications())

  @doc """
  Posts a best-effort notification without requesting authorization.

  Options: `:title`, `:subtitle`, `:body` (UTF-8 strings), and `:sound` (boolean,
  default false). Reusing `tag` replaces its earlier notification. `:ok` means
  the request was queued; OS permissions and launch mode can prevent delivery.
  Subtitle and sound are supported on macOS; other platforms may fold subtitle
  into the body or use their own sound policy.
  """
  def show(tag, opts \\ []) do
    if Keyword.keyword?(opts) &&
         Enum.all?(Keyword.keys(opts), &(&1 in [:title, :subtitle, :body, :sound])) do
      title = Keyword.get(opts, :title, "")
      subtitle = Keyword.get(opts, :subtitle, "")
      body = Keyword.get(opts, :body, "")
      sound = Keyword.get(opts, :sound, false)

      if valid_text?(tag, 512) && tag != "" && valid_text?(title, 1024) &&
           valid_text?(subtitle, 1024) && valid_text?(body, 16384) && is_boolean(sound),
         do:
           completed(
             GPUI.Native.Backend.show_system_notification(tag, title, subtitle, body, sound)
           ),
         else: {:error, :invalid_notification}
    else
      {:error, :invalid_notification}
    end
  end

  @doc "Retracts a delivered or pending notification, best effort."
  def dismiss(tag) do
    if valid_text?(tag, 512) && tag != "",
      do: completed(GPUI.Native.Backend.dismiss_system_notification(tag)),
      else: {:error, :invalid_notification}
  end

  @doc """
  Subscribes the calling process to `{:gpui_system_notification, tag, action_id}`.
  Body activation has a nil action ID. The newest process-wide subscription
  replaces the previous one. Registration does not request authorization.
  """
  def subscribe, do: completed(GPUI.Native.Backend.subscribe_system_notifications())

  defp valid_text?(value, limit),
    do: is_binary(value) && byte_size(value) <= limit && String.valid?(value)

  defp completed({:ok, :ok}), do: :ok
  defp completed({:error, "invalid_notification"}), do: {:error, :invalid_notification}
  defp completed({:error, _} = error), do: error

  defp normalize({:ok, "authorized"}), do: {:ok, :authorized}
  defp normalize({:ok, "denied"}), do: {:ok, :denied}
  defp normalize({:ok, "not_determined"}), do: {:ok, :not_determined}
  defp normalize({:ok, "unsupported"}), do: {:ok, :unsupported}
  defp normalize({:error, _} = error), do: error
end
