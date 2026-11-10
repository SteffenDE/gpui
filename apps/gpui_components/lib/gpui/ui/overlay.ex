defmodule GPUI.UI.Overlay do
  @moduledoc """
  Controlled overlay components backed by GPUI Component.

  Overlay content is declared with ordinary HEEx named slots. Overlay state
  remains authoritative in Elixir assigns while native state preserves focus
  and dismissal behavior between snapshots.
  """

  alias GPUI.Template.Component.Slot
  alias GPUI.Element
  alias GPUI.Components.Schema, as: ComponentsSchema

  require ComponentsSchema

  @type slot :: %{
          required(:attrs) => keyword(),
          required(:children) => [GPUI.Element.child()]
        }

  ComponentsSchema.define_component_option_types(
    tooltip_options: :ui_tooltip,
    dialog_options: :ui_dialog,
    dropdown_menu_options: :ui_dropdown_menu,
    context_menu_options: :ui_context_menu,
    menu_button_options: :ui_menu_button,
    popover_options: :ui_popover
  )

  @anchors ~w(top_left top_center top_right bottom_left bottom_center bottom_right left_center right_center)

  @doc """
  Builds a native tooltip with one `:trigger` and one `:content` slot.
  Content accepts text or display elements such as icons, styled spans and layouts.
  Interactive controls belong in a popover.
  `top_start`/`top_end` and `bottom_start`/`bottom_end` align to trigger edges.

  `delay` is the show delay in milliseconds from `0` through `60_000`. Set `hoverable` when
  the pointer may move into the tooltip without dismissing it.
  `close_delay` controls dismissal after leaving both surfaces (default 500ms);
  zero closes immediately while allowing transfer across an anchored gap.

  `keyboard: true` shows on Tab-originated trigger focus and enables Escape
  dismissal. It does not add a Tab stop. The default is false.

  `placement` defaults to `"cursor"`; `"top"`, `"bottom"`, `"left"` and `"right"`
  anchor to the trigger with `offset` pixels of spacing, flipping and clamping
  within the viewport. `collision_padding` sets the viewport margin in logical
  pixels (default 4, range 0 through 1_000_000). Anchored origins round to the
  nearest physical pixel after fitting; content size stays unchanged.
  `appearance: false` removes native chrome. Styles on
  `:content` apply to the tooltip body, and an optional textual `:shortcut` slot
  carries independently styled accessory text. Set `letter_spacing` on the
  shortcut slot to add pixels of tracking per grapheme, including the final
  trailing advance. Its default is zero and retains ordinary text rendering.
  Nonzero tracking uses a single shaped line with optional liga/clig ligatures
  disabled; inherited kerning, font fallback and text styling are retained.

  #{ComponentsSchema.component_options_doc(:ui_tooltip)}
  """
  @spec tooltip(tooltip_options()) :: Element.t()
  def tooltip(assigns) when is_map(assigns) do
    assigns =
      assigns
      |> ComponentsSchema.apply_defaults(:ui_tooltip)
      |> ComponentsSchema.validate_component_assigns!(:ui_tooltip, [:trigger, :content, :shortcut])

    trigger = one_slot!(assigns, :trigger, :tooltip)
    content = one_slot!(assigns, :content, :tooltip)
    rich = Enum.any?(content.children, &match?(%Element{}, &1))
    text = if rich, do: "", else: tooltip_text!(content.children)
    shortcut = Map.get(assigns, :shortcut, [])

    if length(shortcut) > 1,
      do: raise(ArgumentError, "tooltip accepts at most one :shortcut slot")

    styled = rich or content.attrs != [] or shortcut != [] or not assigns.appearance

    content_nodes =
      if styled,
        do: [
          %Element{
            type: :ui_tooltip_content,
            attrs: Keyword.put(content.attrs, :text, text),
            children: if(rich, do: content.children, else: [])
          }
        ],
        else: []

    shortcut_nodes =
      Enum.map(shortcut, fn slot ->
        ComponentsSchema.validate_component_assigns!(
          Map.new(Keyword.take(slot.attrs, [:letter_spacing])),
          :ui_tooltip_shortcut
        )

        %Element{
          type: :ui_tooltip_shortcut,
          attrs: Keyword.put(slot.attrs, :text, tooltip_text!(slot.children))
        }
      end)

    unless Map.get(assigns, :children, []) == [] do
      raise ArgumentError, "tooltip content must use :trigger and :content named slots"
    end

    unless is_number(assigns.delay) and assigns.delay >= 0 and assigns.delay <= 60_000 and
             is_number(assigns.close_delay) and assigns.close_delay >= 0 and
             assigns.close_delay <= 60_000 and
             is_boolean(assigns.hoverable) and is_boolean(assigns.keyboard) and
             is_boolean(assigns.appearance) and
             assigns.placement in ~w(cursor top top_start top_end bottom bottom_start bottom_end left right) and
             is_number(assigns.offset) and
             assigns.offset >= 0 and is_number(assigns.collision_padding) and
             assigns.collision_padding >= 0 and assigns.collision_padding <= 1.0e6 do
      raise ArgumentError,
            "tooltip needs delay and close_delay between 0 and 60_000, boolean appearance/hoverable/keyboard, a supported placement, nonnegative offset and collision_padding between 0 and 1_000_000"
    end

    id = component_id!(assigns, :ui_tooltip)

    %Element{
      type: :ui_tooltip,
      attrs:
        assigns
        |> Map.drop([:children, :trigger, :content, :shortcut])
        |> Map.put(:id, id)
        |> Map.put(:text, text)
        |> Map.put(:delay, assigns.delay / 1)
        |> Map.put(:close_delay, assigns.close_delay / 1)
        |> Map.to_list(),
      children:
        [
          %Element{type: :ui_tooltip_trigger, attrs: trigger.attrs, children: trigger.children}
        ] ++ content_nodes ++ shortcut_nodes
    }
  end

  @doc """
  Builds a controlled modal dialog with an optional `:trigger` and one `:content` slot.

  A non-empty `title` supplies the dialog heading and names its optional
  trigger. Changes to `open` are emitted through `phx-change`. The native dialog traps
  focus while open and restores the previous focus when it closes, unless another
  element requests focus in the same render. Escape and overlay clicks request
  closure when enabled. Set `pointer_dismiss: false` to keep outside clicks from
  dismissing a confirmation while retaining Escape. Enter belongs to the content (for example, an input's
  `phx-submit`); it does not request closure. Set `open: false` after a successful
  submission to close the dialog.

  Set `show_title: false` for search-first dialogs without a visible heading.
  `title` still names the dialog contract and optional trigger. `margin_top`
  controls the dialog's distance from the top of the window. `margin_top_ratio`
  instead uses a fraction of the current viewport height (0 to 1), including
  after resizing. Supply at most one of these options; omitting both retains
  the native viewport-relative placement.

  `fullscreen: true` fills the current native viewport, removes dialog padding
  and borders, and overrides width and top-margin placement. Content dimensions
  follow native window resizing without requiring application layout updates.

  #{ComponentsSchema.component_options_doc(:ui_dialog)}
  """
  @spec dialog(dialog_options()) :: Element.t()
  def dialog(assigns) when is_map(assigns) do
    assigns =
      assigns
      |> ComponentsSchema.apply_defaults(:ui_dialog)
      |> ComponentsSchema.validate_component_assigns!(:ui_dialog, [:trigger, :content])

    trigger = optional_slot!(assigns, :trigger, :dialog)
    content = one_slot!(assigns, :content, :dialog)

    validate_dialog!(assigns)
    id = component_id!(assigns, :ui_dialog)

    children =
      List.wrap(
        trigger &&
          %Element{type: :ui_dialog_trigger, attrs: trigger.attrs, children: trigger.children}
      ) ++
        [%Element{type: :ui_dialog_content, attrs: content.attrs, children: content.children}]

    %Element{
      type: :ui_dialog,
      attrs:
        assigns
        |> Map.drop([:children, :trigger, :content])
        |> Map.put(:id, id)
        |> Map.put(:width, assigns.width / 1)
        |> Map.to_list(),
      children: children
    }
  end

  @doc """
  Builds a controlled dropdown menu with one `:trigger` and one or more `:item` slots.

  A non-empty `label` names the trigger and its expanded state. `phx-change` receives controlled open-state changes and `phx-select` receives
  the selected item value. Popup-menu keyboard navigation, dismissal, and focus
  restoration are provided by GPUI Component.

  #{ComponentsSchema.component_options_doc(:ui_dropdown_menu)}
  """
  @spec dropdown_menu(dropdown_menu_options()) :: Element.t()
  def dropdown_menu(assigns) when is_map(assigns) do
    assigns =
      assigns
      |> ComponentsSchema.apply_defaults(:ui_dropdown_menu)
      |> ComponentsSchema.validate_component_assigns!(:ui_dropdown_menu, [:trigger, :item])

    trigger = one_slot!(assigns, :trigger, :dropdown_menu)
    items = Map.get(assigns, :item, [])

    validate_dropdown_menu!(assigns, items)
    id = component_id!(assigns, :ui_dropdown_menu)

    children =
      [
        %Element{
          type: :ui_dropdown_menu_trigger,
          attrs: trigger.attrs,
          children: trigger.children
        }
      ] ++
        Enum.map(items, &dropdown_menu_item!/1)

    %Element{
      type: :ui_dropdown_menu,
      attrs:
        assigns
        |> Map.drop([:children, :trigger, :item])
        |> Map.put(:id, id)
        |> Map.to_list(),
      children: children
    }
  end

  @doc """
  Builds a context menu around its content, with one or more `:item` slots.

  A right click on the content opens a compact menu of the items at the
  pointer, and `phx-select` receives the chosen item's value. The pointer and
  the arrow keys move one highlight; Enter chooses, and Escape or a click
  outside closes the menu. Items take the same attributes as `dropdown_menu/1`
  items: disabled items can't be chosen, and checked items show a check mark.
  Primary clicks still reach the content.

  #{ComponentsSchema.component_options_doc(:ui_context_menu)}
  """
  @spec context_menu(context_menu_options()) :: Element.t()
  def context_menu(assigns) when is_map(assigns) do
    assigns =
      assigns
      |> ComponentsSchema.apply_defaults(:ui_context_menu)
      |> ComponentsSchema.validate_component_assigns!(:ui_context_menu, [:item])

    items = Map.get(assigns, :item, [])
    if items == [], do: raise(ArgumentError, "context_menu requires at least one :item slot")

    values = Enum.map(items, &dropdown_menu_item_value!/1)

    if Enum.uniq(values) != values do
      raise ArgumentError, "context_menu item values must be unique"
    end

    id = component_id!(assigns, :ui_context_menu)

    %Element{
      type: :ui_context_menu,
      attrs:
        assigns
        |> Map.drop([:children, :item])
        |> Map.put(:id, id)
        |> Map.to_list(),
      children: Map.get(assigns, :children, []) ++ Enum.map(items, &dropdown_menu_item!/1)
    }
  end

  @doc """
  Builds a button that opens a menu of its `:item` slots.

  The content is the button. Pressing it opens the same compact menu as
  `context_menu/1`, below the button: `anchor` picks which corner of the menu
  lines up with it (`"top_left"` or `"top_right"`). `phx-select` receives the
  chosen item's value, and `label` names the button for accessibility. The
  press doesn't reach enclosing click handlers, so the button can sit inside a
  clickable row. While its menu is open the button stays visible, even when
  its style hides it until a group is hovered.

  #{ComponentsSchema.component_options_doc(:ui_menu_button)}
  """
  @spec menu_button(menu_button_options()) :: Element.t()
  def menu_button(assigns) when is_map(assigns) do
    assigns =
      assigns
      |> ComponentsSchema.apply_defaults(:ui_menu_button)
      |> ComponentsSchema.validate_component_assigns!(:ui_menu_button, [:item])

    items = Map.get(assigns, :item, [])
    if items == [], do: raise(ArgumentError, "menu_button requires at least one :item slot")

    values = Enum.map(items, &dropdown_menu_item_value!/1)

    if Enum.uniq(values) != values do
      raise ArgumentError, "menu_button item values must be unique"
    end

    id = component_id!(assigns, :ui_menu_button)

    %Element{
      type: :ui_menu_button,
      attrs:
        assigns
        |> Map.drop([:children, :item])
        |> Map.put(:id, id)
        |> Map.to_list(),
      children: Map.get(assigns, :children, []) ++ Enum.map(items, &dropdown_menu_item!/1)
    }
  end

  @doc """
  Builds a controlled popover with one `:trigger` and one `:content` slot.

  Set `open_on_hover` for pointer activation, with `hover_delay` and `close_delay`
  in milliseconds. Crossing from the trigger into the content keeps it open.
  Set `focus_on_open: false` for previews that preserve the current input focus.
  Set `trigger_passthrough: true` when the trigger child owns focus and
  activation. The wrapper then leaves Tab, Enter/Space, and pointer activation
  to that control. Pair it with `open_on_hover: true` for hover cards decorating
  links without adding a second focus stop or toggling the card on link activation.
  The default `false` retains the wrapper's focusable button behavior.

  Supply all four `anchor_x`, `anchor_y`, `anchor_width`, and `anchor_height`
  values for a window-relative virtual anchor, such as a rich-text link span.
  `anchor_hovered` controls its hover timer without adding a pointer-blocking
  trigger. The trigger slot remains required but may contain an empty div.
  For virtual anchors, `parent_popover` names an enclosing hover preview so clicks inside a deferred child
  do not dismiss its parent. Escape dismisses the most recently opened preview.

  A non-empty `label` names the trigger and its expanded state. Changes to `open` are emitted through `phx-change`. Escape and, by default,
  outside clicks request closure and restore focus to the trigger.

  #{ComponentsSchema.component_options_doc(:ui_popover)}
  """
  @spec popover(popover_options()) :: Element.t()
  def popover(assigns) when is_map(assigns) do
    assigns =
      assigns
      |> ComponentsSchema.apply_defaults(:ui_popover)
      |> ComponentsSchema.validate_component_assigns!(:ui_popover, [:trigger, :content])

    trigger = one_slot!(assigns, :trigger, :popover)
    content = one_slot!(assigns, :content, :popover)

    unless Map.get(assigns, :children, []) == [] do
      raise ArgumentError, "popover content must use :trigger and :content named slots"
    end

    unless is_boolean(assigns.open) and is_boolean(assigns.appearance) and
             is_boolean(assigns.closable) and is_boolean(assigns.open_on_hover) and
             is_boolean(assigns.focus_on_open) and is_boolean(assigns.trigger_passthrough) do
      raise ArgumentError,
            "popover open, appearance, closable, open_on_hover, focus_on_open, and trigger_passthrough must be booleans"
    end

    unless assigns.anchor in @anchors do
      raise ArgumentError,
            "popover anchor must be one of #{Enum.map_join(@anchors, ", ", &inspect/1)}"
    end

    unless Enum.all?(
             [assigns.hover_delay, assigns.close_delay],
             &(is_number(&1) and &1 >= 0 and &1 <= 60_000)
           ) do
      raise ArgumentError,
            "popover hover_delay and close_delay must be between 0 and 60_000 milliseconds"
    end

    rectangle =
      Enum.map([:anchor_x, :anchor_y, :anchor_width, :anchor_height], &Map.get(assigns, &1))

    unless Enum.all?(rectangle, &is_nil/1) or
             (Enum.all?(rectangle, &(is_number(&1) and abs(&1) <= 1_000_000)) and
                Enum.at(rectangle, 2) >= 0 and Enum.at(rectangle, 3) >= 0) do
      raise ArgumentError,
            "popover virtual anchors require four finite window coordinates and nonnegative dimensions"
    end

    parent = Map.get(assigns, :parent_popover)

    unless parent == nil or (is_binary(parent) and byte_size(parent) in 1..256),
      do: raise(ArgumentError, "popover parent_popover must be a non-empty component id")

    unless is_boolean(assigns.anchor_hovered),
      do: raise(ArgumentError, "popover anchor_hovered must be boolean")

    id = component_id!(assigns, :ui_popover)

    %Element{
      type: :ui_popover,
      attrs:
        assigns
        |> Map.drop([:children, :trigger, :content])
        |> Map.put(:id, id)
        |> Map.to_list(),
      children: [
        %Element{type: :ui_popover_trigger, attrs: trigger.attrs, children: trigger.children},
        %Element{type: :ui_popover_content, attrs: content.attrs, children: content.children}
      ]
    }
  end

  defp validate_dropdown_menu!(assigns, items) do
    unless Map.get(assigns, :children, []) == [] do
      raise ArgumentError, "dropdown_menu content must use :trigger and :item named slots"
    end

    unless is_boolean(assigns.open) and is_boolean(assigns.disabled) do
      raise ArgumentError, "dropdown_menu open and disabled must be booleans"
    end

    unless assigns.anchor in @anchors do
      raise ArgumentError,
            "dropdown_menu anchor must be one of #{Enum.map_join(@anchors, ", ", &inspect/1)}"
    end

    if items == [], do: raise(ArgumentError, "dropdown_menu requires at least one :item slot")

    values = Enum.map(items, &dropdown_menu_item_value!/1)

    if Enum.uniq(values) != values do
      raise ArgumentError, "dropdown_menu item values must be unique"
    end
  end

  defp dropdown_menu_item!(%Slot{} = slot) do
    attrs = Map.new(slot.attrs)
    value = dropdown_menu_item_value!(slot)
    disabled = Map.get(attrs, :disabled, false)
    checked = Map.get(attrs, :checked, false)

    unless is_boolean(disabled) and is_boolean(checked) do
      raise ArgumentError, "dropdown_menu item disabled and checked must be booleans"
    end

    label = textual_slot!(slot.children, "dropdown_menu :item")

    %Element{
      type: :ui_dropdown_menu_item,
      attrs:
        attrs
        |> Map.put(:value, value)
        |> Map.put(:label, label)
        |> Map.put(:disabled, disabled)
        |> Map.put(:checked, checked)
        |> Map.to_list(),
      children: []
    }
  end

  defp dropdown_menu_item_value!(%Slot{attrs: attrs}) do
    case Keyword.get(attrs, :value) do
      value when is_binary(value) and value != "" -> value
      _other -> raise ArgumentError, "dropdown_menu :item requires a non-empty string value"
    end
  end

  defp validate_dialog!(assigns) do
    unless Map.get(assigns, :children, []) == [] do
      raise ArgumentError, "dialog content must use :trigger and :content named slots"
    end

    boolean_attrs =
      Enum.map([:open, :show_title, :overlay, :closable, :keyboard, :close_button], &assigns[&1])

    unless Enum.all?(boolean_attrs, &is_boolean/1) do
      raise ArgumentError,
            "dialog open, show_title, overlay, closable, keyboard, and close_button must be booleans"
    end

    unless is_number(assigns.width) and assigns.width > 0 and assigns.width <= 4096 do
      raise ArgumentError, "dialog width must be greater than 0 and at most 4096"
    end

    margin_top = assigns[:margin_top]
    ratio = assigns[:margin_top_ratio]

    unless is_nil(ratio) or (is_number(ratio) and ratio >= 0 and ratio <= 1),
      do: raise(ArgumentError, "dialog margin_top_ratio must be between 0 and 1")

    if not is_nil(ratio) and not is_nil(margin_top),
      do: raise(ArgumentError, "dialog accepts margin_top or margin_top_ratio, not both")

    unless is_nil(margin_top) or
             (is_number(margin_top) and margin_top >= 0 and margin_top <= 4096) do
      raise ArgumentError, "dialog margin_top must be between 0 and 4096"
    end
  end

  defp tooltip_text!(children), do: textual_slot!(children, "tooltip :content")

  defp textual_slot!(children, description) do
    text = Enum.map_join(children, &textual_fragment!(&1, description))

    if text == "", do: raise(ArgumentError, "#{description} must contain text"), else: text
  end

  defp textual_fragment!(%Element{type: :text, children: children}, description),
    do: Enum.map_join(children, &textual_fragment!(&1, description))

  defp textual_fragment!(value, _description) when is_binary(value), do: value

  defp textual_fragment!(value, _description) when is_number(value) or is_atom(value),
    do: to_string(value)

  defp textual_fragment!(value, description),
    do: raise(ArgumentError, "#{description} must be textual, got: #{inspect(value)}")

  defp component_id!(assigns, type) do
    id = Map.get(assigns, :id)

    if is_binary(id) and id != "" do
      id
    else
      name = type |> Atom.to_string() |> String.trim_leading("ui_")

      raise ArgumentError,
            "GPUI.UI.Overlay.#{name}/1 requires :id to be a non-empty string; " <>
              "got: #{inspect(id)}"
    end
  end

  defp optional_slot!(assigns, name, component) do
    case Map.get(assigns, name, []) do
      [] -> nil
      [%Slot{} = slot] -> slot
      _other -> raise ArgumentError, "#{component} accepts at most one :#{name} slot"
    end
  end

  defp one_slot!(assigns, name, component) do
    case Map.get(assigns, name, []) do
      [%Slot{} = slot] -> slot
      _other -> raise ArgumentError, "#{component} requires exactly one :#{name} slot"
    end
  end
end
