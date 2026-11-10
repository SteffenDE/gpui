defmodule GPUI.Event do
  @moduledoc """
  Normalized UI event delivered from a display into `GPUI.Session`.

  Physical primitive clicks with held keys include a top-level `:modifiers` map
  with boolean `:control`, `:alt`, `:shift`, and `:platform` fields. Unmodified
  clicks and keyboard/accessibility activation retain the plain click envelope.
  `:pointer` events carry `:value` with boolean `:hovered` and the same four
  modifier fields, including key changes while the pointer is stationary.
  Root `phx-keycapture` callbacks emit `:keystroke` values with an additional
  `:captured` boolean. `true` means native handling was stopped; `false` preserves
  native handling, including editable input and unlisted or modified shortcuts.
  """

  alias GPUI.Transfer.Event, as: TransferEvent

  defstruct [:type, :window_id, :event, :value, attrs: %{}]

  @type type ::
          :keystroke
          | :text
          | :paste
          | :resize
          | :pointer
          | :link_hover
          | :click
          | :command
          | :change
          | :open
          | :select
          | :release
          | :search
          | :submit
          | :range
          | :link
          | :transaction
          | :selection
          | :viewport
          | :geometry
          | :range_geometry
          | :hit_test
          | :bounds
          | :focus
          | :blur
          | :keydown
          | :keyup
          | :drag_enter
          | :drag_move
          | :drag_leave
          | :drop
          | :clipboard
          | :clipboard_write
          | :copy
          | :file_read
          | :window_close_request
          | :window_focus
          | :window_blur
          | :window_closed
          | :window_geometry
          | :window_appearance
  @type payload :: %{
          required(:type) => type(),
          required(:window_id) => pos_integer(),
          optional(:event) => String.t(),
          optional(:value) => term(),
          optional(atom()) => term()
        }

  @type t :: %__MODULE__{
          type: type(),
          window_id: pos_integer() | nil,
          event: String.t() | nil,
          value: term(),
          attrs: map()
        }

  @routed_types [
    :keystroke,
    :text,
    :paste,
    :resize,
    :pointer,
    :link_hover,
    :click,
    :command,
    :change,
    :open,
    :select,
    :release,
    :search,
    :submit,
    :range,
    :link,
    :transaction,
    :selection,
    :viewport,
    :geometry,
    :range_geometry,
    :hit_test,
    :bounds,
    :focus,
    :blur,
    :keydown,
    :keyup,
    :drag_enter,
    :drag_move,
    :drag_leave,
    :drop,
    :clipboard,
    :clipboard_write,
    :copy,
    :file_read
  ]
  @lifecycle_types [
    :window_close_request,
    :window_focus,
    :window_blur,
    :window_closed,
    :window_geometry,
    :window_appearance
  ]
  @injectable_routed_types [
    :click,
    :command,
    :change,
    :open,
    :release,
    :search,
    :submit,
    :keydown,
    :keyup
  ]
  @injectable_types @injectable_routed_types ++
                      (@lifecycle_types -- [:window_appearance, :window_geometry])
  @known_types @routed_types ++ @lifecycle_types
  @max_event_name_bytes 512
  @max_file_read_bytes 25 * 1_024 * 1_024

  @doc "Returns the renderer-independent event types routed to root view callbacks."
  @spec routed_types() :: [type()]
  def routed_types, do: @routed_types

  defguard is_routed_type(type) when type in @routed_types

  @doc "Returns event types accepted by the low-level native injection boundary."
  @spec injectable_types() :: [type()]
  def injectable_types, do: @injectable_types

  @spec normalize(t() | map() | keyword()) :: {:ok, payload()} | {:error, term()}
  def normalize(%__MODULE__{} = event), do: event |> to_map() |> normalize()

  def normalize(event) when is_list(event) do
    if Keyword.keyword?(event), do: event |> Map.new() |> normalize(), else: invalid(:shape)
  end

  def normalize(%{type: type} = event) when type in @known_types do
    with :ok <- validate_window_id(event),
         :ok <- validate_event_name(type, event) do
      normalize_value(type, event)
    end
  end

  def normalize(%{type: type}) when is_atom(type), do: {:error, {:unsupported_event_type, type}}
  def normalize(%{type: _type}), do: invalid(:type)
  def normalize(event) when is_map(event), do: invalid(:type)
  def normalize(_event), do: invalid(:shape)

  defp validate_window_id(%{window_id: window_id})
       when is_integer(window_id) and window_id > 0,
       do: :ok

  defp validate_window_id(_event), do: invalid(:window_id)

  defp validate_event_name(type, event) when type in @routed_types do
    case Map.fetch(event, :event) do
      {:ok, name}
      when is_binary(name) and name != "" and byte_size(name) <= @max_event_name_bytes ->
        if String.valid?(name), do: :ok, else: invalid(:event)

      _invalid ->
        invalid(:event)
    end
  end

  defp validate_event_name(type, event) when type in @lifecycle_types do
    if Map.has_key?(event, :event), do: invalid(:event), else: :ok
  end

  defp normalize_value(
         :window_geometry,
         %{value: %{bounds: [x, y, width, height], initial: initial}} = event
       )
       when is_number(x) and is_number(y) and is_number(width) and is_number(height) and width > 0 and
              height > 0 and is_boolean(initial),
       do: {:ok, event}

  defp normalize_value(:window_geometry, _), do: invalid(:value)

  defp normalize_value(:window_appearance, %{value: mode} = event) when mode in [:light, :dark],
    do: {:ok, event}

  defp normalize_value(:window_appearance, _), do: invalid(:value)

  defp normalize_value(type, event) when type in [:drag_enter, :drag_move, :drag_leave, :drop] do
    with {:ok, value} <- fetch_value(event),
         {:ok, value} <- TransferEvent.normalize(type, value) do
      {:ok, Map.put(event, :value, value)}
    end
  end

  defp normalize_value(:clipboard, event) do
    with {:ok, value} <- fetch_value(event),
         true <- is_map(value) do
      {:ok, Map.put(event, :value, GPUI.Transfer.Payload.new(value))}
    else
      _invalid -> invalid(:value)
    end
  rescue
    ArgumentError -> invalid(:value)
  end

  defp normalize_value(:file_read, event) do
    with {:ok, value} <- fetch_value(event),
         true <- valid_file_read_value?(value) do
      {:ok, event}
    else
      _invalid -> invalid(:value)
    end
  end

  defp normalize_value(:link_hover, %{value: value} = event) when is_map(value) do
    if is_binary(value[:id]) and is_boolean(value[:hovered]) and
         (is_nil(value[:link]) or is_binary(value[:link])) and
         Enum.all?([:x, :y, :width, :height], &finite_number?(value[&1])) and
         value.width >= 0 and value.height >= 0, do: {:ok, event}, else: invalid(:value)
  end

  defp normalize_value(:link_hover, _), do: invalid(:value)

  defp normalize_value(:pointer, %{value: value} = event) when is_map(value) do
    if Enum.all?([:hovered, :control, :alt, :shift, :platform], &is_boolean(Map.get(value, &1))),
      do: {:ok, event},
      else: invalid(:value)
  end

  defp normalize_value(:pointer, _), do: invalid(:value)

  defp normalize_value(:click, event) do
    cond do
      Map.has_key?(event, :value) ->
        invalid(:value)

      not Map.has_key?(event, :modifiers) ->
        {:ok, event}

      is_map(event.modifiers) and
          Enum.all?(
            [:control, :alt, :shift, :platform],
            &is_boolean(Map.get(event.modifiers, &1))
          ) ->
        {:ok, event}

      true ->
        invalid(:modifiers)
    end
  end

  defp normalize_value(type, event) when type in [:command, :clipboard_write, :copy] do
    if Map.has_key?(event, :value), do: invalid(:value), else: {:ok, event}
  end

  defp normalize_value(type, event) when type in @lifecycle_types do
    if Map.has_key?(event, :value), do: invalid(:value), else: {:ok, event}
  end

  defp normalize_value(:change, event), do: require_value(event, &valid_change_value?/1)

  defp normalize_value(:select, event),
    do: require_value(event, &(is_binary(&1) or is_nil(&1)))

  defp normalize_value(type, event) when type in [:search, :submit, :link, :open],
    do: require_value(event, &is_binary/1)

  defp normalize_value(:release, event), do: require_value(event, &finite_number?/1)

  defp normalize_value(type, event) when type in [:keydown, :keyup],
    do: require_value(event, &is_binary/1)

  defp normalize_value(:keystroke, event), do: require_value(event, &valid_keystroke?/1)
  defp normalize_value(:resize, event), do: require_value(event, &valid_terminal_size?/1)

  defp normalize_value(type, event) when type in [:text, :paste],
    do: require_value(event, &bounded_string?(&1, 1_048_576, true))

  defp normalize_value(:range, event), do: require_value(event, &valid_range?/1)
  defp normalize_value(:transaction, event), do: require_value(event, &is_map/1)
  defp normalize_value(:selection, event), do: require_value(event, &is_list/1)
  defp normalize_value(:viewport, event), do: require_value(event, &is_map/1)
  defp normalize_value(:geometry, event), do: require_value(event, &is_map/1)
  defp normalize_value(:range_geometry, event), do: require_value(event, &is_list/1)
  defp normalize_value(:hit_test, event), do: require_value(event, &is_map/1)
  defp normalize_value(:bounds, event), do: require_value(event, &is_map/1)

  defp normalize_value(type, event) when type in [:focus, :blur],
    do: require_value(event, &valid_focus?/1)

  defp require_value(event, validator) do
    with {:ok, value} <- fetch_value(event),
         true <- validator.(value) do
      {:ok, event}
    else
      _invalid -> invalid(:value)
    end
  end

  defp fetch_value(event), do: Map.fetch(event, :value)

  defp valid_keystroke?(%{key: key} = value) do
    bounded_string?(key, 128, false) and
      Enum.all?([:control, :alt, :shift, :platform, :repeat], &is_boolean(Map.get(value, &1)))
  end

  defp valid_keystroke?(_), do: false

  defp valid_terminal_size?(%{columns: cols, rows: rows, cell_width: width, cell_height: height}) do
    is_integer(cols) and cols in 1..512 and is_integer(rows) and rows in 1..256 and
      finite_number?(width) and width > 0 and finite_number?(height) and height > 0
  end

  defp valid_terminal_size?(_), do: false

  defp valid_change_value?(value) when is_binary(value) or is_boolean(value) or is_nil(value),
    do: true

  defp valid_change_value?(value) when is_number(value), do: finite_number?(value)

  defp valid_change_value?(value) when is_list(value) do
    Enum.all?(value, &is_binary/1) or Enum.all?(value, &finite_number?/1)
  end

  defp valid_change_value?(_value), do: false

  defp valid_range?(%{first: first, last: last}),
    do: is_integer(first) and first >= 0 and is_integer(last) and last >= first

  defp valid_range?(_value), do: false

  defp valid_focus?(%{id: id}), do: is_binary(id) and id != "" and String.valid?(id)
  defp valid_focus?(_value), do: false

  defp valid_file_read_value?(%{operation_id: id, status: :cancelled}),
    do: valid_operation_id?(id)

  defp valid_file_read_value?(%{operation_id: id, status: :error, reason: reason}),
    do: valid_operation_id?(id) and bounded_string?(reason, 4_096, true)

  defp valid_file_read_value?(%{status: :selected} = value) do
    id = Map.get(value, :operation_id)
    name = Map.get(value, :name)
    size = Map.get(value, :size)
    data = Map.get(value, :data)

    valid_operation_id?(id) and bounded_string?(name, 4_096, false) and is_integer(size) and
      size >= 0 and is_binary(data) and byte_size(data) == size and size <= @max_file_read_bytes
  end

  defp valid_file_read_value?(_value), do: false
  defp valid_operation_id?(id), do: is_integer(id) and id >= 0

  defp bounded_string?(value, max, allow_empty) do
    is_binary(value) and (allow_empty or value != "") and byte_size(value) <= max and
      String.valid?(value)
  end

  defp finite_number?(value) when is_integer(value), do: true

  defp finite_number?(value) when is_float(value) do
    value |> :erlang.float_to_binary() |> then(&(&1 not in ["nan", "inf", "-inf"]))
  end

  defp finite_number?(_value), do: false
  defp invalid(field), do: {:error, {:invalid_event, field}}

  @doc "Converts an event struct and its extra attributes into a plain event map."
  @spec to_map(t()) :: payload()
  def to_map(%__MODULE__{} = event) do
    event
    |> Map.from_struct()
    |> Map.delete(:attrs)
    |> Map.merge(event.attrs)
    |> Enum.reject(fn {_key, value} -> is_nil(value) end)
    |> Map.new()
  end
end
