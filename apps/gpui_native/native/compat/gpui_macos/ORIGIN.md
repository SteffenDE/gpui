# Vendored GPUI macOS platform

Source: Zed 1662f5f3f6497c5f80830ccdca1edfd1fc0c6c6a (crates/gpui_macos), Apache-2.0.
See LICENSE-APACHE. Workspace manifest dependencies and lints are expanded.
Local changes preserve global logical coordinates across display layouts.

`LOCAL_CHANGES.patch` records every changed/added Rust file against the pinned
upstream source. Display bounds keep their global origin. Visible work areas and
window frames flip y using the primary screen's top edge; initial placement uses
the inverse conversion and selects the screen containing the requested center.
Display backing scale is exposed for physical-pixel center calculations.

The upstream Cocoa implementation uses deprecated APIs; the compatibility crate
allows that existing deprecation lint while retaining other compiler warnings.
Permission reads and explicit requests also use UserNotifications callbacks without
blocking the main thread. Unbundled launches return Unsupported before accessing
the notification center. Existing platform tests cover coordinate conversion;
SDK and app integration tests cover the permission bridge and launch guard.

The options delivery path sets subtitle and optional default sound without
requesting authorization. Foreground sound follows the content's sound field.
A real bundled Mac test verified visible title/subtitle/body and activation in Elixir.

Window activation restores a minimized window before making it key and ordering
it to the front. The SDK reports native roots' initial activation state as well
as subsequent changes; real bundled tests exercise independent runtimes and
notification-banner routing into existing and newly created windows.
