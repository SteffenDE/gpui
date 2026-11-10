defmodule GPUI.WindowSpec do
  @moduledoc """
  Declarative window specification returned by the application DSL.

  Each window owns a bounded set of platform-aware `GPUI.Command` bindings in
  addition to its title, size, and root view.
  """

  @max_key_bytes 128
  @max_title_bytes 512

  @type root :: {module(), map() | keyword()}
  @type key :: String.t()
  @type chrome :: :system | :content
  @type t :: %__MODULE__{
          id: pos_integer() | nil,
          key: key() | nil,
          title: String.t(),
          size: {pos_integer(), pos_integer()} | nil,
          min_size: {pos_integer(), pos_integer()} | nil,
          resizable: boolean(),
          chrome: chrome(),
          appearance_events: boolean(),
          geometry_events: boolean(),
          center_in_work_area: boolean(),
          restore_bounds: {number(), number(), number(), number()} | nil,
          cascade_offset: non_neg_integer(),
          root: root() | nil,
          commands: [GPUI.Command.t()]
        }

  @enforce_keys [:title]
  defstruct [
    :id,
    :key,
    :title,
    :size,
    :min_size,
    :root,
    resizable: true,
    chrome: :system,
    restore_bounds: nil,
    cascade_offset: 0,
    geometry_events: false,
    center_in_work_area: false,
    appearance_events: false,
    commands: []
  ]

  @doc "Builds and validates a declarative window specification."
  @spec new(String.t(), keyword()) :: t()
  def new(title, opts \\ []) when is_binary(title) and is_list(opts) do
    root = Keyword.get(opts, :root)

    root =
      case root do
        nil -> nil
        {module, assigns} -> {module, Map.new(assigns)}
        module when is_atom(module) -> {module, %{}}
      end

    opts
    |> Keyword.put(:title, title)
    |> Keyword.put(:root, root)
    |> then(&struct!(__MODULE__, &1))
    |> validate!()
  end

  @doc "Validates a declarative window specification and returns it."
  @spec validate!(t()) :: t()
  def validate!(
        %__MODULE__{
          title: title,
          key: key,
          size: size,
          min_size: min_size,
          resizable: resizable,
          chrome: chrome,
          root: root,
          commands: commands
        } = window
      ) do
    validate_title!(title)
    validate_key!(key)

    unless valid_size?(size) do
      raise ArgumentError, "window size must contain positive integer width and height"
    end

    unless valid_size?(min_size) do
      raise ArgumentError, "window minimum size must contain positive integer width and height"
    end

    unless is_boolean(resizable), do: raise(ArgumentError, "window resizable must be a boolean")

    unless chrome in [:system, :content],
      do: raise(ArgumentError, "window chrome must be :system or :content")

    unless valid_root?(root) do
      raise ArgumentError, "window root must contain a module and map or keyword assigns"
    end

    unless is_boolean(window.appearance_events),
      do: raise(ArgumentError, "window appearance_events must be a boolean")

    unless valid_bounds?(window.restore_bounds),
      do:
        raise(
          ArgumentError,
          "window restore_bounds must contain finite x, y, width and height with positive size"
        )

    unless is_integer(window.cascade_offset) and window.cascade_offset >= 0,
      do: raise(ArgumentError, "window cascade_offset must be a nonnegative integer")

    unless is_boolean(window.geometry_events),
      do: raise(ArgumentError, "window geometry_events must be a boolean")

    unless is_boolean(window.center_in_work_area),
      do: raise(ArgumentError, "window center_in_work_area must be a boolean")

    GPUI.Command.validate_all!(commands)

    if match?({_, _}, root) do
      {module, _assigns} = root

      unless Code.ensure_loaded?(module) and function_exported?(module, :render, 1) do
        raise ArgumentError, "window root #{inspect(module)} must implement render/1"
      end
    end

    window
  end

  defp valid_bounds?(nil), do: true

  defp valid_bounds?({x, y, width, height})
       when is_number(x) and is_number(y) and is_number(width) and is_number(height),
       do: width > 0 and height > 0 and Enum.all?([x, y, width, height], &(abs(&1) < 1.0e9))

  defp valid_bounds?(_), do: false

  defp validate_title!(title) when is_binary(title) and byte_size(title) <= @max_title_bytes,
    do: :ok

  defp validate_title!(title) when is_binary(title),
    do: raise(ArgumentError, "window title must be at most #{@max_title_bytes} bytes")

  defp validate_title!(_title), do: raise(ArgumentError, "window title must be a string")

  defp validate_key!(nil), do: :ok

  defp validate_key!(key)
       when is_binary(key) and key != "" and byte_size(key) <= @max_key_bytes,
       do: :ok

  defp validate_key!(key) when is_binary(key),
    do: raise(ArgumentError, "window key must contain 1 through #{@max_key_bytes} bytes")

  defp validate_key!(_key), do: raise(ArgumentError, "window key must be a non-empty string")

  defp valid_size?(nil), do: true
  defp valid_size?({width, height}), do: positive_integer?(width) and positive_integer?(height)
  defp valid_size?(_size), do: false

  defp valid_root?(nil), do: true

  defp valid_root?({module, assigns}),
    do: is_atom(module) and (is_map(assigns) or is_list(assigns))

  defp valid_root?(_root), do: false

  defp positive_integer?(value), do: is_integer(value) and value > 0
end
