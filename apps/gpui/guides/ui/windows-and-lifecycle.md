# Windows and lifecycle

Window topology remains application-owned. Window specifications declare
platform constraints, while optional view callbacks decide how lifecycle
requests affect application state.

## Window lifecycle

Window declarations contain only declarative platform constraints:

```elixir
window "Workspace" do
  size 1100, 720
  min_size 760, 480
  resizable true
  chrome :content
  root WorkspaceView
end
```

Views opt into lifecycle handling through an idiomatic optional behaviour
callback rather than arbitrary routing strings:

```elixir
def handle_window_event(:close_request, _event, assigns) do
  {:noreply, %{assigns | close_dialog_open: true}}
end

def handle_window_event(:focus, _event, assigns) do
  {:noreply, %{assigns | window_active: true}}
end

def handle_window_event(:blur, _event, assigns) do
  {:noreply, %{assigns | window_active: false}}
end
```

Exporting `handle_window_event/3` enables interception and activation delivery
for the view's window. A close request remains asynchronous. Return
`{:noreply, assigns}` to keep the window open—for example while rendering an
application-owned confirmation dialog—or `{:close, assigns}` to approve closure.
Ordinary `handle_event/3` handlers may also return `{:close, assigns}` after a
confirmation action.

`min_size`, `resizable`, and `chrome` are declarative creation options interpreted by the
native display. `chrome :system` is the default; `chrome :content` extends
application content into native chrome and enables ordinary containers marked
with `window_control="drag|close|maximize|minimize"`. Creation-option changes
reopen the native window while retaining its session ID. See
[Decision: declarative native window chrome](window-chrome.html)
for platform behavior and fallback rules. The existing `:window_closed` event remains an internal final
notification after a window actually closes. Window `:focus` and `:blur`
callbacks report native activation and are separate from element focus.

## Restoring and cascading native windows

`restore_bounds({x, y, width, height})` supplies a frame in logical display
coordinates. The native display restores it only if it still fits a connected
display's visible work area (with one logical pixel of rounding slack); otherwise
it uses the ordinary `size` and centered placement. `cascade_offset(36)` offsets
a valid restored frame, wrapping to the work area's inset corner when the result
crosses its right or bottom edge. The default offset is zero.

`geometry_events(true)` opts in to `handle_window_event(:geometry, event, assigns)`.
The event's `value` is `%{bounds: [x, y, width, height], initial: boolean}`. The
initial frame establishes local state; subsequent bounds changes can be persisted
by the application. Identical frames are deduplicated, and fullscreen frames are
excluded. Persistence is application-owned; GPUI does not write a settings file.
`center_in_work_area(true)` centers a fresh or fallback window within the primary
display's visible work area, using the display backing scale to truncate its
center offset in physical pixels. Restored bounds take precedence. The default
remains full-display centering.

The pinned Mac compatibility platform preserves global display origins and uses
the primary screen's top edge to convert AppKit frames. Coordinate unit tests
cover left/right/above/below display layouts. A physical single-display comparison
cannot establish actual multi-monitor or display-disconnect behavior.

## Notification authorization

With `gpui_native`, `GPUI.SystemNotifications.status/0` reads OS authorization
without prompting. `request/0` explicitly asks for permission; call it only after
user activation of a notification control. Both return `{:ok, status}` or
`{:error, reason}` and wait for an OS callback, so invoke them in supervised tasks
rather than blocking a root view handler. Status is `:authorized`, `:denied`,
`:not_determined`, or `:unsupported`.

The macOS bridge uses UserNotifications and returns `:unsupported` before
accessing that framework when the process is outside an app bundle. Application
identity alone does not create a bundle or confer another app's permission.
Other platforms currently report `:unsupported` through this permission API.
`show(tag, title: ..., subtitle: ..., body: ..., sound: false)` queues a
best-effort delivery without asking for permission. Reusing a tag replaces its
previous notification; `dismiss(tag)` retracts pending and delivered notifications
where the platform supports it. A successful request does not establish that the
OS displayed the notification. macOS supports subtitle and sound; other platforms
may fold subtitle into body text or use their own sound policy.

Call `subscribe/0` from a long-lived process to receive
`{:gpui_system_notification, tag, action_id}` when the user activates a notification.
Body activation has a nil action ID. There is one process-wide subscription;
the newest replaces the previous one. Application code owns target lookup,
window selection, and navigation. Subscription does not request authorization.
