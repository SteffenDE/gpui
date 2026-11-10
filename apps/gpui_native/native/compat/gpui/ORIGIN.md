# Vendored GPUI core

Source: https://github.com/zed-industries/zed/tree/1662f5f3f6497c5f80830ccdca1edfd1fc0c6c6a/crates/gpui
Revision: the 1662f5f3f6497c5f80830ccdca1edfd1fc0c6c6a revision pinned in the SDK Cargo.lock.
License: Apache-2.0; see LICENSE-APACHE and source notices.

The source, build script, and resources are copied from the pinned checkout.
Examples and benches are omitted. Workspace package/dependency/lint fields are
expanded in Cargo.toml. Workspace path dependencies retain the SDK's existing
explicit Zed Git revision shared with the SDK native crates. GPUI macros
are also vendored at that revision so gpui-component uses matching macros.

Local extension: request_precise_measured_layout retains measured leaf lengths
before Taffy layout. The existing request_measured_layout remains pixel-snapped.
This supports fractional prose advances without changing other native controls.

StyledText.with_fractional_line_height retains fractional line advances through
its standard shaping, wrapping, and truncation path. LOCAL_CHANGES.patch records
all changed upstream Rust source files; the other Rust sources are unchanged.

PlatformDisplay.scale_factor defaults to 1.0 for initial-placement pixel alignment.
The patched Mac platform overrides it using the display backing scale.

Notification authorization queries and explicit requests return asynchronous tasks,
with an Unsupported default for platforms that do not implement permissions.

Notification presentation options add subtitle and sound without changing existing
notification struct initializers. The new macOS options path never requests
permission implicitly; activation callbacks return through the foreground executor.
