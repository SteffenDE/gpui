defmodule GPUI.Schema.Core do
  @moduledoc """
  Neutral primitive declarations included by every native host.

  `text_input` keeps `phx-keydown` and `phx-keyup` as string callbacks. Optional
  `phx-keystroke` and `phx-keyrelease` callbacks emit `:keystroke` events with
  canonical `key`, `control`, `alt`, `shift`, `platform`, and `repeat` fields.
  Releases report `repeat: false`; held key-down events retain their repeat flag.
  `text_input.capture_keys` captures selected unmodified named keys before native
  editing or focus traversal, emitting its normal key-down callbacks once.
  `"shift-tab"` explicitly captures reverse focus traversal as a modifier-bearing
  `tab` callback. Other modified keys retain their native behavior. The default
  empty list captures nothing.
  `phx-pointer-position` reports physical pointer movement inside a `div` or
  primitive `button` as a change event with `[x, y]` in logical window pixels.
  Pair it with `phx-pointer` to clear the last position when the pointer leaves.
  A changed positive `select_all_request` selects the input's current text once;
  subsequent controlled value updates do not reselect it.
  `div` and `button` support `phx-pointer` observations of pointer enter/leave
  and modifier changes while hovered, independent of keyboard focus. The value
  contains boolean `hovered`, `control`, `alt`, `shift`, and `platform` fields.
  Use a stable `id` for these observations. `clipboard_text` with
  `phx-clipboard-write` writes text on activation and acknowledges the write.
  A changed positive `clipboard_write_request` on a stable `div` or primitive
  `button` writes `clipboard_text` once during rendering and acknowledges through
  `phx-clipboard-write`. This supports application shortcuts without changing
  focus or synthesizing a click. A positive request disables activation-triggered
  copying on that element. Zero does not reset the last handled request.
  `clipboard_without_primary` skips the write while Command (macOS) or Control
  (other platforms) is held, leaving `phx-click` available for link actions.
  A window's root `div` can use `phx-keycapture` with `capture_keys` to intercept
  listed unmodified keys and explicitly listed `shift-` keys before window commands.
  For example, `"c"` captures plain C and `"shift-c"` captures Shift-C. While enabled, it reports all
  keystrokes with an extra `captured` boolean: only intercepted keys stop native
  handling. Unlisted modifiers and keys in an editable input report `captured: false`
  and retain native behavior. Remove the callback to disable interception.
  A window's root `div` can set `phx-window-keyrelease` to observe structured
  key releases before focused descendants handle them. This callback does not
  capture or suppress native editing and does not require a focus request.
  A primitive `scroll` can set `scroll_horizontal: true` to scroll along X,
  controlled by `scroll_left` and `scroll_request`. Its scroll event reports
  `[left, viewport_width, maximum_left]`; vertical scrolling retains
  `[top, viewport_height, maximum_top]`. Scrollbars follow the selected axis.
  `div` and primitive `button` can use `phx-pointer-down` to capture a left
  mouse press. The change event contains `[x, y]` in logical window pixels.
  It suppresses native mouse activation and bubbling for that press; `phx-click`
  remains available for keyboard and accessibility activation. Disabled controls
  do not capture presses. Root `div` callbacks `phx-window-pointer-position` and
  `phx-window-pointer-up` observe movement and left-button release anywhere in the
  window, including outside their bounds, with the same coordinate payload.
  They do not suppress other native mouse handling.
  `div`, `item`, and `button` accept `tab_stop: false` to skip sequential Tab
  navigation while retaining explicit focus requests and focus callbacks.
  Focus-enabled containers with a stable `id` can set `focus_outline_color`,
  `focus_outline_width` (default 2px), `focus_outline_offset` (default 1px), and
  `focus_outline_radius` (default 0px) in `style`. The outline paints outside
  the box only for keyboard focus, without changing layout or hit bounds.
  `focus_outline_mode: :tab` restricts it to Tab navigation: arrows, Enter,
  Space, Escape, and modifiers preserve the ring; other keys and pointer
  presses clear it. The default mode follows any keyboard input.
  A `div` can use `phx-keyrelease` for keys while that region itself owns focus;
  keys handled by a focused child control do not activate the region callback.
  """
  @behaviour GPUI.Schema.Provider
  @components [
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [rem_size: :positive_number],
      children: true,
      events: [],
      extension: nil,
      kind: :viewport,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :viewport
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [
        capture_keys: :string_list,
        clipboard_text: :string,
        clipboard_write_request: {:default, :non_negative_integer, 0},
        clipboard_without_primary: {:default, :boolean, false},
        window_control: {:enum, ["drag", "close", "maximize", "minimize"]},
        accessibility_role:
          {:enum,
           [
             "button",
             "checkbox",
             "dialog",
             "group",
             "heading",
             "image",
             "label",
             "link",
             "list",
             "list_item",
             "menu",
             "menu_item",
             "menu_item_checkbox",
             "progress",
             "radio",
             "slider",
             "splitter",
             "switch",
             "tab",
             "tab_list",
             "tab_panel",
             "text",
             "textbox",
             "tree",
             "tree_item"
           ]},
        accessibility_label: :accessibility_label,
        accessibility_description: :accessibility_description,
        accessibility_value: :accessibility_value,
        accessibility_selected: :boolean,
        accessibility_expanded: :boolean,
        accessibility_checked: :accessibility_checked,
        accessibility_orientation: {:enum, ["horizontal", "vertical"]},
        accessibility_disabled: :boolean,
        id: :string,
        motion_request: {:default, :non_negative_integer, 0},
        motion_duration: {:default, :positive_integer, 180},
        motion_delay: {:default, :non_negative_integer, 0},
        motion_easing:
          {:default, {:enum, ["linear", "ease_in", "ease_out", "ease_in_out"]}, "ease_out"},
        motion_policy: {:default, {:enum, ["respect_system", "disabled"]}, "respect_system"},
        motion_from_opacity: {:default, :number, 1.0},
        motion_from_x: {:default, :number, 0.0},
        motion_from_y: {:default, :number, 0.0},
        motion_to_opacity: {:default, :number, 1.0},
        motion_to_x: {:default, :number, 0.0},
        motion_to_y: {:default, :number, 0.0},
        motion_repeat: {:default, :boolean, false},
        motion_alternate: {:default, :boolean, false},
        focus_request: {:default, :non_negative_integer, 0},
        tab_stop: {:default, :boolean, true}
      ],
      children: false,
      events: [
        click: :"phx-click",
        pointer: :"phx-pointer",
        pointer_position: :"phx-pointer-position",
        pointer_down: :"phx-pointer-down",
        clipboard_write: :"phx-clipboard-write",
        bounds: :"phx-bounds-change",
        keycapture: :"phx-keycapture",
        window_keyrelease: :"phx-window-keyrelease",
        window_pointer_position: :"phx-window-pointer-position",
        window_pointer_up: :"phx-window-pointer-up",
        keyrelease: :"phx-keyrelease",
        focus: :"phx-focus",
        blur: :"phx-blur"
      ],
      extension: nil,
      kind: :container,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :div
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [
        clipboard_text: :string,
        clipboard_write_request: {:default, :non_negative_integer, 0},
        clipboard_without_primary: {:default, :boolean, false},
        window_control: {:enum, ["drag", "close", "maximize", "minimize"]},
        accessibility_role:
          {:enum,
           [
             "button",
             "checkbox",
             "dialog",
             "group",
             "heading",
             "image",
             "label",
             "link",
             "list",
             "list_item",
             "menu",
             "menu_item",
             "menu_item_checkbox",
             "progress",
             "radio",
             "slider",
             "splitter",
             "switch",
             "tab",
             "tab_list",
             "tab_panel",
             "text",
             "textbox",
             "tree",
             "tree_item"
           ]},
        accessibility_label: :accessibility_label,
        accessibility_description: :accessibility_description,
        accessibility_value: :accessibility_value,
        accessibility_selected: :boolean,
        accessibility_expanded: :boolean,
        accessibility_checked: :accessibility_checked,
        accessibility_orientation: {:enum, ["horizontal", "vertical"]},
        accessibility_disabled: :boolean,
        id: :string,
        motion_request: {:default, :non_negative_integer, 0},
        motion_duration: {:default, :positive_integer, 180},
        motion_delay: {:default, :non_negative_integer, 0},
        motion_easing:
          {:default, {:enum, ["linear", "ease_in", "ease_out", "ease_in_out"]}, "ease_out"},
        motion_policy: {:default, {:enum, ["respect_system", "disabled"]}, "respect_system"},
        motion_from_opacity: {:default, :number, 1.0},
        motion_from_x: {:default, :number, 0.0},
        motion_from_y: {:default, :number, 0.0},
        motion_to_opacity: {:default, :number, 1.0},
        motion_to_x: {:default, :number, 0.0},
        motion_to_y: {:default, :number, 0.0},
        motion_repeat: {:default, :boolean, false},
        motion_alternate: {:default, :boolean, false},
        focus_request: {:default, :non_negative_integer, 0},
        tab_stop: {:default, :boolean, true}
      ],
      children: false,
      events: [
        click: :"phx-click",
        pointer: :"phx-pointer",
        pointer_position: :"phx-pointer-position",
        pointer_down: :"phx-pointer-down",
        clipboard_write: :"phx-clipboard-write",
        bounds: :"phx-bounds-change",
        focus: :"phx-focus",
        blur: :"phx-blur"
      ],
      extension: nil,
      kind: :container,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :button
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [
        id: :required_string,
        anchor:
          {:default,
           {:enum,
            [
              "top_left",
              "top_center",
              "top_right",
              "bottom_left",
              "bottom_center",
              "bottom_right",
              "left_center",
              "right_center"
            ]}, "top_left"},
        position_mode: {:default, {:enum, ["local", "window"]}, "local"},
        position_x: :number,
        position_y: :number,
        offset_x: {:default, :number, 0.0},
        offset_y: {:default, :number, 0.0},
        fit:
          {:default, {:enum, ["switch_anchor", "snap_to_window", "snap_with_margin"]},
           "switch_anchor"},
        margin: {:default, :non_negative_number, 0.0},
        priority: {:default, :layer_priority, 0}
      ],
      children: true,
      events: [],
      extension: nil,
      kind: :anchored_layer,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :layer
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [
        window_control: {:enum, ["drag", "close", "maximize", "minimize"]},
        accessibility_role:
          {:enum,
           [
             "button",
             "checkbox",
             "dialog",
             "group",
             "heading",
             "image",
             "label",
             "link",
             "list",
             "list_item",
             "menu",
             "menu_item",
             "menu_item_checkbox",
             "progress",
             "radio",
             "slider",
             "splitter",
             "switch",
             "tab",
             "tab_list",
             "tab_panel",
             "text",
             "textbox",
             "tree",
             "tree_item"
           ]},
        accessibility_label: :accessibility_label,
        accessibility_description: :accessibility_description,
        accessibility_value: :accessibility_value,
        accessibility_selected: :boolean,
        accessibility_expanded: :boolean,
        accessibility_checked: :accessibility_checked,
        accessibility_orientation: {:enum, ["horizontal", "vertical"]},
        accessibility_disabled: :boolean,
        id: :string,
        motion_request: {:default, :non_negative_integer, 0},
        motion_duration: {:default, :positive_integer, 180},
        motion_delay: {:default, :non_negative_integer, 0},
        motion_easing:
          {:default, {:enum, ["linear", "ease_in", "ease_out", "ease_in_out"]}, "ease_out"},
        motion_policy: {:default, {:enum, ["respect_system", "disabled"]}, "respect_system"},
        motion_from_opacity: {:default, :number, 1.0},
        motion_from_x: {:default, :number, 0.0},
        motion_from_y: {:default, :number, 0.0},
        motion_to_opacity: {:default, :number, 1.0},
        motion_to_x: {:default, :number, 0.0},
        motion_to_y: {:default, :number, 0.0},
        motion_repeat: {:default, :boolean, false},
        motion_alternate: {:default, :boolean, false}
      ],
      children: false,
      events: [click: :"phx-click", bounds: :"phx-bounds-change"],
      extension: nil,
      kind: :container,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :span
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [
        window_control: {:enum, ["drag", "close", "maximize", "minimize"]},
        accessibility_role:
          {:enum,
           [
             "button",
             "checkbox",
             "dialog",
             "group",
             "heading",
             "image",
             "label",
             "link",
             "list",
             "list_item",
             "menu",
             "menu_item",
             "menu_item_checkbox",
             "progress",
             "radio",
             "slider",
             "splitter",
             "switch",
             "tab",
             "tab_list",
             "tab_panel",
             "text",
             "textbox",
             "tree",
             "tree_item"
           ]},
        accessibility_label: :accessibility_label,
        accessibility_description: :accessibility_description,
        accessibility_value: :accessibility_value,
        accessibility_selected: :boolean,
        accessibility_expanded: :boolean,
        accessibility_checked: :accessibility_checked,
        accessibility_orientation: {:enum, ["horizontal", "vertical"]},
        accessibility_disabled: :boolean,
        id: :string,
        scrollbar: {:default, :boolean, false},
        scroll_top: {:default, :number, 0.0},
        scroll_left: {:default, :number, 0.0},
        scroll_horizontal: {:default, :boolean, false},
        scroll_request: {:default, :non_negative_integer, 0},
        motion_request: {:default, :non_negative_integer, 0},
        motion_duration: {:default, :positive_integer, 180},
        motion_delay: {:default, :non_negative_integer, 0},
        motion_easing:
          {:default, {:enum, ["linear", "ease_in", "ease_out", "ease_in_out"]}, "ease_out"},
        motion_policy: {:default, {:enum, ["respect_system", "disabled"]}, "respect_system"},
        motion_from_opacity: {:default, :number, 1.0},
        motion_from_x: {:default, :number, 0.0},
        motion_from_y: {:default, :number, 0.0},
        motion_to_opacity: {:default, :number, 1.0},
        motion_to_x: {:default, :number, 0.0},
        motion_to_y: {:default, :number, 0.0},
        motion_repeat: {:default, :boolean, false},
        motion_alternate: {:default, :boolean, false}
      ],
      children: false,
      events: [click: :"phx-click", bounds: :"phx-bounds-change", scroll: :"phx-scroll"],
      extension: nil,
      kind: :container,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :scroll
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [
        window_control: {:enum, ["drag", "close", "maximize", "minimize"]},
        accessibility_role:
          {:enum,
           [
             "button",
             "checkbox",
             "dialog",
             "group",
             "heading",
             "image",
             "label",
             "link",
             "list",
             "list_item",
             "menu",
             "menu_item",
             "menu_item_checkbox",
             "progress",
             "radio",
             "slider",
             "splitter",
             "switch",
             "tab",
             "tab_list",
             "tab_panel",
             "text",
             "textbox",
             "tree",
             "tree_item"
           ]},
        accessibility_label: :accessibility_label,
        accessibility_description: :accessibility_description,
        accessibility_value: :accessibility_value,
        accessibility_selected: :boolean,
        accessibility_expanded: :boolean,
        accessibility_checked: :accessibility_checked,
        accessibility_orientation: {:enum, ["horizontal", "vertical"]},
        accessibility_disabled: :boolean,
        id: :string,
        motion_request: {:default, :non_negative_integer, 0},
        motion_duration: {:default, :positive_integer, 180},
        motion_delay: {:default, :non_negative_integer, 0},
        motion_easing:
          {:default, {:enum, ["linear", "ease_in", "ease_out", "ease_in_out"]}, "ease_out"},
        motion_policy: {:default, {:enum, ["respect_system", "disabled"]}, "respect_system"},
        motion_from_opacity: {:default, :number, 1.0},
        motion_from_x: {:default, :number, 0.0},
        motion_from_y: {:default, :number, 0.0},
        motion_to_opacity: {:default, :number, 1.0},
        motion_to_x: {:default, :number, 0.0},
        motion_to_y: {:default, :number, 0.0},
        motion_repeat: {:default, :boolean, false},
        motion_alternate: {:default, :boolean, false}
      ],
      children: false,
      events: [click: :"phx-click", bounds: :"phx-bounds-change"],
      extension: nil,
      kind: :container,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :list
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [
        window_control: {:enum, ["drag", "close", "maximize", "minimize"]},
        accessibility_role:
          {:enum,
           [
             "button",
             "checkbox",
             "dialog",
             "group",
             "heading",
             "image",
             "label",
             "link",
             "list",
             "list_item",
             "menu",
             "menu_item",
             "menu_item_checkbox",
             "progress",
             "radio",
             "slider",
             "splitter",
             "switch",
             "tab",
             "tab_list",
             "tab_panel",
             "text",
             "textbox",
             "tree",
             "tree_item"
           ]},
        accessibility_label: :accessibility_label,
        accessibility_description: :accessibility_description,
        accessibility_value: :accessibility_value,
        accessibility_selected: :boolean,
        accessibility_expanded: :boolean,
        accessibility_checked: :accessibility_checked,
        accessibility_orientation: {:enum, ["horizontal", "vertical"]},
        accessibility_disabled: :boolean,
        id: :string,
        motion_request: {:default, :non_negative_integer, 0},
        motion_duration: {:default, :positive_integer, 180},
        motion_delay: {:default, :non_negative_integer, 0},
        motion_easing:
          {:default, {:enum, ["linear", "ease_in", "ease_out", "ease_in_out"]}, "ease_out"},
        motion_policy: {:default, {:enum, ["respect_system", "disabled"]}, "respect_system"},
        motion_from_opacity: {:default, :number, 1.0},
        motion_from_x: {:default, :number, 0.0},
        motion_from_y: {:default, :number, 0.0},
        motion_to_opacity: {:default, :number, 1.0},
        motion_to_x: {:default, :number, 0.0},
        motion_to_y: {:default, :number, 0.0},
        motion_repeat: {:default, :boolean, false},
        motion_alternate: {:default, :boolean, false}
      ],
      children: false,
      events: [click: :"phx-click", bounds: :"phx-bounds-change"],
      extension: nil,
      kind: :container,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :item
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [
        id: :required_string,
        buffer: :text_buffer,
        focus_request: {:default, :non_negative_integer, 0},
        disabled: :boolean,
        soft_wrap: :boolean,
        auto_grow: :boolean,
        min_lines: {:default, :positive_integer, 1},
        max_lines: {:default, :positive_integer, 8},
        submit_policy: {:default, {:enum, ["newline", "submit"]}, "newline"},
        show_whitespaces: :boolean,
        tab_size: {:default, :positive_integer, 2},
        hard_tabs: :boolean,
        line_numbers: :boolean,
        geometry_ranges: :text_ranges,
        scroll_request: {:default, :non_negative_integer, 0},
        scroll_to: :text_position,
        decorations: :text_decorations,
        style_runs: :text_style_runs,
        inline_projections: :text_inline_projections,
        block_projections: :text_block_projections
      ],
      children: false,
      events: [
        transaction: :"phx-transaction",
        keystroke: :"phx-keystroke",
        keyrelease: :"phx-keyrelease",
        submit: :"phx-submit",
        selection: :"phx-selection-change",
        viewport: :"phx-viewport-change",
        geometry: :"phx-geometry-change",
        range_geometry: :"phx-range-geometry-change",
        hit_test: :"phx-hit-test",
        focus: :"phx-focus",
        blur: :"phx-blur"
      ],
      extension: nil,
      kind: :text_surface,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: true,
      tag: :text_surface
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [
        id: :string,
        value: :string,
        placeholder: :string,
        capture_keys:
          {:default,
           {:enum_list, ["tab", "shift-tab", "enter", "escape", "up", "down", "backspace"]}, []},
        focus_request: {:default, :non_negative_integer, 0},
        select_all_request: {:default, :non_negative_integer, 0}
      ],
      children: false,
      events: [
        change: :"phx-change",
        keydown: :"phx-keydown",
        keyup: :"phx-keyup",
        keystroke: :"phx-keystroke",
        keyrelease: :"phx-keyrelease",
        focus: :"phx-focus",
        blur: :"phx-blur"
      ],
      extension: nil,
      kind: :input,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :text_input
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [
        data: :required_string,
        id: :string,
        spin_duration: {:default, :non_negative_integer, 0}
      ],
      children: false,
      events: [],
      extension: nil,
      kind: :svg,
      public_hidden_attrs: [],
      public_required_attrs: [:data],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :svg
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [raster: :resource, label: :string],
      children: false,
      events: [],
      extension: nil,
      kind: :image,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :img
    },
    %{
      __struct__: GPUI.Schema.Component,
      attrs: [truncate_start: :boolean],
      children: false,
      events: [],
      extension: nil,
      kind: :text,
      public_hidden_attrs: [],
      public_required_attrs: [],
      public_slots: [],
      required_events: [],
      stateful: false,
      tag: :text
    }
  ]
  @impl true
  @spec components() :: [GPUI.Schema.Component.t()]
  def components do
    @components
  end
end
