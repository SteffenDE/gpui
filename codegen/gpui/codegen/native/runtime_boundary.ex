defmodule GPUI.Codegen.Native.RuntimeBoundary do
  @moduledoc "Defines RustQ-owned native runtime lifecycle NIF boundaries."

  use RustQ.Native,
    otp_app: :gpui_native,
    build: false,
    load: false,
    rust_sources: ["apps/gpui_native/native/src/nif.rs"]

  alias RustQ.Type, as: R

  @spec host_info() :: R.nif_result(term())
  defnif(host_info(), do: host_info_impl(nif_env()))

  @spec set_app_identity(String.t(), String.t()) :: R.nif_result(term())
  defnif set_app_identity(identifier, name) do
    set_app_identity_impl(nif_env(), identifier, name)
  end

  @nif schedule: :dirty_io
  @spec system_notification_status() :: R.nif_result(term())
  defnif(system_notification_status(), do: system_notification_status_impl(nif_env()))

  @nif schedule: :dirty_io
  @spec request_system_notifications() :: R.nif_result(term())
  defnif(request_system_notifications(), do: request_system_notifications_impl(nif_env()))

  @nif schedule: :dirty_io
  @spec show_system_notification(String.t(), String.t(), String.t(), String.t(), boolean()) ::
          R.nif_result(term())
  defnif(show_system_notification(tag, title, subtitle, body, sound),
    do: show_system_notification_impl(nif_env(), tag, title, subtitle, body, sound)
  )

  @nif schedule: :dirty_io
  @spec dismiss_system_notification(String.t()) :: R.nif_result(term())
  defnif(dismiss_system_notification(tag), do: dismiss_system_notification_impl(nif_env(), tag))

  @nif schedule: :dirty_io
  @spec subscribe_system_notifications() :: R.nif_result(term())
  defnif(subscribe_system_notifications(), do: subscribe_system_notifications_impl(nif_env()))

  @nif schedule: :dirty_io
  @spec start_runtime() :: R.nif_result(term())
  defnif(start_runtime(), do: start_runtime_impl(nif_env()))

  @nif schedule: :dirty_io
  @spec stop_runtime(R.resource(RuntimeResource.t())) :: R.nif_result(term())
  defnif(stop_runtime(runtime), do: stop_runtime_impl(nif_env(), runtime))
end
