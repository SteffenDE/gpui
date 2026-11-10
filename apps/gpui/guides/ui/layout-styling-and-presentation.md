# Layout, styling, and presentation

Static design values use Tailwind-compatible classes. Runtime-derived native
geometry uses explicit bounded attributes rather than dynamic CSS expressions.

## Hover groups

A named group can change a descendant's visibility or background while the
pointer is anywhere inside the group:

```elixir
<div class="group/row hover:bg-[#1d1f24]">
  <text class="bg-[#16191d] group-hover/row:bg-[#1d1f24]">3</text>
  <button class="invisible group-hover/row:visible">More</button>
</div>
```

Unnamed groups use `group` and `group-hover:bg-*`. Background utilities accept
the same colors and alpha modifiers as ordinary `bg-*` utilities. The explicit
style form is `group_hover_background: {"row", {:rgb, 0x1D1F24}}`.
Named class groups normalize to bounded strings; explicit atom group names
remain supported at the native boundary. Hover styling stays in the native
renderer and does not require application hover events.

## Ordinary positioning versus anchored layers

The default positioning model remains HTML-like composition with
Tailwind-compatible classes. Keep content in the ordinary element tree whenever
normal layout and paint order are sufficient:

```elixir
<div class="relative w-full h-full">
  <div class="absolute top-2 right-2">...</div>
</div>
```

Use the neutral `<layer>` primitive only as a native top-layer/portal escape
hatch when content must paint after its ancestors or remain fitted to the native
window. A layer accepts exactly one child and does not own open state,
dismissal, focus, or menu behavior. It is not a replacement for `relative`,
`absolute`, or inset utilities.

```elixir
<layer
  id="completion-popup"
  anchor="bottom_left"
  position_mode="window"
  position_x={assigns.anchor_x}
  position_y={assigns.anchor_y}
  offset_y={6}
  fit="snap_with_margin"
  margin={8}
  priority={10}
>
  <div class="w-[320px] rounded-md bg-slate-800">...</div>
</layer>
```

All static presentation inside the layer—dimensions, spacing, colors, borders,
and typography—still belongs in classes. Runtime-derived native geometry stays
in explicit attributes such as `position_x` and `position_y`.

`position_mode="window"` interprets `position_x` and `position_y` as native
window-relative pixels. `position_mode="local"` interprets them relative to the
layer's normal tree location; omitted coordinates use that location directly.
Anchors identify which corner or edge-center of the child meets the position.
The fit strategy either switches anchors when possible, snaps to the window, or
snaps with a uniform margin. Priority is bounded to `0..1024`; it maps to GPUI's
deferred paint order rather than pretending to be CSS `z-index`. Consequently,
`z-*`, `fixed`, and similar CSS utilities remain unsupported instead of being
mapped to different GPUI behavior. Enum values retain the project's established
underscore convention (`bottom_left`, `snap_with_margin`), while element
composition and static styling remain HTML/Tailwind-like.

## Motion

Ordinary containers support bounded entrance motion with a stable `id` and a
monotonic request token:

```heex
<div
  id="saved-notice"
  motion_request={assigns.saved_notice_motion}
  motion_duration={180}
  motion_easing="ease_out"
  motion_from_opacity={0.0}
  motion_from_y={8}
>
  ...
</div>
```

The native display interpolates opacity and window-native-pixel x/y offsets to
the ordinary destination presentation. Reusing the token does not restart the
animation. Motion respects GPUI's reduced-motion preference and emits no
completion event; application behavior must not depend on presentation timing.
See [Decision: declarative native motion](declarative-motion.html)
for bounds, ownership, interruption, and remote-display behavior.

Use `motion_to_opacity`, `motion_to_x`, and `motion_to_y` for explicit destinations.
`motion_repeat={true}` repeats the duration; `motion_alternate={true}` travels
from the starting values to the destinations and back within each duration.
`ease_in_out` uses CSS cubic-bezier(0.42, 0, 0.58, 1). Offsets remain pixels;
responsive consumers derive them from their measured viewport.

SVGs without an explicit foreground inherit the surrounding text color at
paint time, including ancestor hover colors. Explicit SVG colors override that
inheritance. Foreground and background hover styles can be used together; the
native generator combines them into one refinement.

Inline SVGs accept `id` and `spin_duration` in milliseconds (`0` disables rotation).
Rotation runs clockwise about the SVG center and respects reduced motion. Stable
IDs retain animation phase across ordinary view updates. Repeating presentation
runs in the native frame loop without application timers.

Native tree payloads are limited to 128 nested child levels. Deeper trees return
`native_tree_too_deep`; the window/session remains usable. Tree decoding uses an
owned Rustler environment and an explicit Rust worker stack so deeply composed
views do not exhaust a BEAM scheduler thread's stack.

## Element bounds

Ordinary containers can opt into asynchronous native bounds events when a
consumer needs to anchor a layer to actual layout rather than guessed
coordinates:

```elixir
<div id="completion-anchor" phx-bounds-change="anchor-bounds-changed">
  ...
</div>
```

The event includes the stable element ID and window-relative native-pixel
geometry:

```elixir
%{
  type: :bounds,
  value: %{
    id: "completion-anchor",
    x: 240.0,
    y: 118.0,
    width: 96.0,
    height: 28.0,
    coordinate_space: "window_native_pixels"
  }
}
```

Observation is opt-in and requires a non-empty `id`. Native events are emitted
only after layout, deduplicated while geometry is unchanged, and updated after
rerender or resize. The event is asynchronous; it does not expose GPUI layout
IDs or provide a synchronous measurement NIF. Use the resulting geometry as
runtime `<layer>` coordinates while keeping static presentation in classes.

## Styling

Templates normalize a constrained Tailwind-compatible vocabulary into typed
native style attributes. Static layout and design values should use classes;
reserve `style` for runtime values that cannot be known in the template.

Views import compile-time hexadecimal color sigils for explicit style values:

```elixir
style={[background: ~RGB"0f172a", border_color: ~RGBA"ffffff1a"]}
```

`~RGB` accepts three or six hexadecimal digits; `~RGBA` accepts four or eight.
The sigil name replaces the leading `#`, and short literals expand each digit.
They produce the same `{:rgb, value}` and `{:rgba, value}` values accepted by
the canonical style schema.

Supported groups include:

- flex display, direction, wrapping, alignment, growth, shrink, the
  `flex-1`, `flex-auto`, `flex-initial`, and `flex-none` shorthands, and basis
  values such as `basis-1/2` or `basis-[240px]`;
- relative and absolute positioning with pixel, percentage, fractional, full, or
  `auto` insets through `inset-*`, `inset-x-*`, `inset-y-*`, `top-*`,
  `right-*`, `bottom-*`, and `left-*`; numeric and arbitrary pixel/percentage
  insets also accept Tailwind's leading-negative form;
- foreground, background, and border colors from the complete Tailwind CSS
  3.4.17 default palette, plus `black`, `white`, and `transparent`; named colors
  accept percentage alpha modifiers such as `bg-black/40` and
  `text-slate-300/70`;
- bounded arbitrary hexadecimal colors in three-, four-, six-, or eight-digit
  form, such as `bg-[#abc]`, `text-[#abcd]`, and `border-[#101828]`; arbitrary
  colors also accept percentage alpha modifiers;
- typography, weight, line height, alignment, whitespace, ellipsis/truncation,
  and opacity, including safe arbitrary pixel and numeric values;
- padding, margin, and gaps using Tailwind's numeric spacing convention where
  one unit is four pixels, including decimal values such as `gap-1.5`;
- width, height, and minimum/maximum dimensions using spacing values, arbitrary
  pixels, percentages, or fractions such as `w-1/2`;
- borders, border colors, and radius, including exact arbitrary pixel radii;
- hidden overflow clipping and native cursor feedback.

```elixir
<div class="relative flex flex-1 basis-1/2 min-h-0 gap-1.5 p-4 overflow-hidden bg-[#101828]">
  <text class="absolute top-2 right-2 text-[#94a3b8] text-xs">Draft</text>
  <text class="truncate text-[#f8fafc] text-[13px] leading-[18px] font-semibold">
    Settings
  </text>
</div>
```

`<list>` and `<item>` carry no hidden flex, gap, or padding policy; declare
layout explicitly with classes. `<span>` is likewise a neutral container.

Use the semantic `<scroll>` element for scrolling. An overflow utility is not
accepted as a substitute because GPUI scrolling requires native scroll state and
behavior. Unsupported variants and CSS expressions, such as `hover:*`,
`mx-auto`, or `w-[calc(...)]`, follow `GPUI.Tailwind`'s configured unknown-class
policy instead of being silently reinterpreted. Programmatic trees explicitly
import helpers such as `GPUI.Builder.style/3`, `GPUI.Builder.px/1`, and
`GPUI.Builder.rgb/1`.

## Themes

The component theme belongs to the process-global native display environment and
refreshes every open native window:

```elixir
{:ok, display} = GPUI.Display.Native.start_link(theme: :dark)
:ok = GPUI.Display.Native.set_theme(display, :light)
```

See `GPUI.UI` for the complete option-level API and
[Overlays and menus](overlays-and-menus.html) for named-slot components.


## Vertical scroll requests

An ordinary `<scroll>` keeps a native vertical scroll offset while it is mounted.
Use a stable `id`, set `scroll_top` in logical pixels from the content's top,
and increment `scroll_request` to request a new position. A zero request leaves
the position under native wheel control. Changing only `scroll_top` does not
move the viewport; rerenders preserve the current position. Requested offsets
are clamped to the content's available extent.

```heex
<scroll id="discussion" scroll_top={assigns.top}
  scroll_request={assigns.request} phx-scroll="discussion_scrolled"
  class="grow min-h-0">
  <div class="flex flex-col">...</div>
</scroll>
```

`phx-scroll` receives a change event with `[top, viewport_height, maximum_top]`
in logical pixels when those values change, including wheel movement and
content or viewport resizing. `phx-bounds-change` can report the viewport's
window-relative position independently. Together with descendant bounds, these
values let the application reveal a target without estimating text heights.
Set `scrollbar: true` for an interactive vertical scrollbar. Overflow reserves
10 logical pixels from the content width; fitting content has no gutter or
thumb. The six-pixel thumb uses the container's foreground at 20% opacity
(35% while hovered or dragged). It supports wheel scrolling, track paging with
a 40-pixel overlap (at least 80% of the viewport), and thumb dragging with
pointer capture. Scrollbar presses preserve the existing focus.

The scrollbar is off by default. The container does not choose an application
focus policy. Use a new ID when the scroll content represents a different target.


### Fractional computed bounds

`style={[preserve_layout_bounds: true]}` preserves fractional computed
border-box positions and dimensions for an element and its descendants.
For example, a 24px control centered in the 35px inner height of a 36px
header with a 1px bottom border starts at y5.5. Hit testing, accessibility
bounds, and painting use that same fractional box. Siblings outside the
subtree retain the normal device-pixel edge snapping.

This is an opt-in final-bounds policy. Authored lengths (including border
and padding metrics) and measured leaf sizes retain their existing snapping
policy. It does not disable every form of pixel snapping or change default
rendering elsewhere.


## Outside rings and shadows

Use `ring-1 ring-slate-300/10` for an outside ring and `shadow-md` for
Tailwind-compatible drop shadows. Supported shadow sizes are `2xs`, `xs`,
`sm`, `md`, `lg`, `xl`, `2xl`, and `none`. These decorations do not change
layout, content padding, or pointer hit areas. They remain subject to ancestor
paint clipping. A ring requires a color; its default width is one logical pixel.

Explicit styles accept a list of shadow layers in CSS paint order (the first
layer appears on top):

```elixir
<div style={[
  ring_width: {:px, 1.0},
  ring_color: {:rgba, 0xD7DBE11A},
  box_shadow: [
    {0.0, 4.0, 6.0, -1.0, {:rgba, 0x0000001A}},
    {0.0, 2.0, 4.0, -2.0, {:rgba, 0x0000001A}}
  ]
]}>...</div>
```

Each tuple is `{offset_x, offset_y, blur_radius, spread_radius, color}`.
Lengths use logical CSS pixels. Offsets and spread may be negative; blur must
be nonnegative, and every number must remain finite in the native renderer.
The renderer converts the CSS blur radius to Gaussian sigma internally.
`box_shadow: []` clears shadow layers. An outside ring follows the element's
corner radii and paints between the shadows and the element background.


### Paint-bound rounding

`style={[paint_rounding: :half_up]}` rounds rectangular paint edges to the
nearest device pixel with ties toward positive infinity. This applies to the
subtree's backgrounds, borders, outside rings and SVG/image rectangles
that use the shared bounds snapper. Content masks retain their covering policy. It leaves layout, hit testing, stroke-width
snapping and glyph rasterization unchanged. Use `preserve_layout_bounds: true`
when computed fractional geometry must reach this paint step.

The default remains half-toward-zero. Descendants inherit an explicit policy;
`paint_rounding: :half_toward_zero` resets it for a nested subtree. The policy
is restored after the subtree paints, so siblings keep their own rounding.

## Rich tooltip cards

`GPUI.UI.Overlay.tooltip/1` accepts ordinary display elements in its content
slot, including vectors, styled text and layout containers. Text-only content
and shortcut slots retain their existing behavior. Use a popover for interactive
controls. Rich content updates refresh an already visible tooltip.

Use `placement: "top_start"` or `"top_end"` to align above a trigger's left or
right edge; `bottom_start` and `bottom_end` provide matching lower placements.
Alignment is preserved when the side flips, with ordinary viewport clamping.
`close_delay` defaults to 500ms for hoverable tooltips; zero dismisses immediately
after leaving both the trigger and popup. Anchored hoverable tooltips allow
pointer transfer through the gap between overlapping trigger and popup edges.
