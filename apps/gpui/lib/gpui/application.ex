defmodule GPUI.Application do
  @moduledoc """
  Behaviour and DSL for OTP-supervised GPUI applications.

  `mount/1` declares a session's initial windows. Interactive state belongs to
  each root view's assigns rather than an unused application-level state value.
  Window blocks may bind stable command IDs with `shortcut/2`; native and remote
  displays dispatch them to the same view handlers used by buttons and menus.
  """

  alias GPUI.WindowSpec

  @doc "Builds the initial renderer-independent window set for a session."
  @callback mount(term()) :: {:ok, [WindowSpec.t()]}
  @callback identity() :: GPUI.Application.Identity.t()

  @optional_callbacks identity: 0

  defmacro __using__(_opts) do
    quote do
      @behaviour GPUI.Application

      import GPUI.Application,
        only: [
          window: 2,
          window: 3,
          size: 2,
          min_size: 2,
          resizable: 1,
          chrome: 1,
          appearance_events: 1,
          geometry_events: 1,
          center_in_work_area: 1,
          restore_bounds: 1,
          cascade_offset: 1,
          root: 1,
          root: 2,
          shortcut: 2
        ]

      def child_spec(opts) do
        %{
          id: Keyword.get(opts, :id, Keyword.get(opts, :name, __MODULE__)),
          start: {GPUI.Runtime, :start_link, [Keyword.put(opts, :app, __MODULE__)]}
        }
      end

      defoverridable child_spec: 1
    end
  end

  @doc "Returns and validates an application's declared identity, when present."
  @spec identity(module()) :: GPUI.Application.Identity.t() | nil
  def identity(app) when is_atom(app) do
    if function_exported?(app, :identity, 0) do
      app.identity() |> GPUI.Application.Identity.validate!()
    end
  end

  @doc "Builds a keyed window specification from a DSL block."
  defmacro window(key, title, do: block) do
    window_ast(key, title, block)
  end

  @doc "Builds a window specification from a DSL block."
  defmacro window(title, do: block) do
    window_ast(nil, title, block)
  end

  defp window_ast(key, title, block) do
    entries = block_entries(block)

    quote do
      Enum.reduce(unquote(entries), %GPUI.WindowSpec{key: unquote(key), title: unquote(title)}, fn
        {:size, width, height}, spec ->
          %{spec | size: {width, height}}

        {:min_size, width, height}, spec ->
          %{spec | min_size: {width, height}}

        {:resizable, resizable}, spec ->
          %{spec | resizable: resizable}

        {:chrome, chrome}, spec ->
          %{spec | chrome: chrome}

        {:center_in_work_area, enabled}, spec ->
          %{spec | center_in_work_area: enabled}

        {:geometry_events, enabled}, spec ->
          %{spec | geometry_events: enabled}

        {:restore_bounds, bounds}, spec ->
          %{spec | restore_bounds: bounds}

        {:cascade_offset, offset}, spec ->
          %{spec | cascade_offset: offset}

        {:appearance_events, enabled}, spec ->
          %{spec | appearance_events: enabled}

        {:root, module, assigns}, spec ->
          %{spec | root: {module, Map.new(assigns)}}

        {:command, id, shortcut}, spec ->
          %{spec | commands: spec.commands ++ [GPUI.Command.new(id, shortcut)]}
      end)
    end
  end

  @doc "Declares a window size inside `window`."
  defmacro size(width, height), do: quote(do: {:size, unquote(width), unquote(height)})

  @doc "Declares a minimum window size inside `window`."
  defmacro min_size(width, height), do: quote(do: {:min_size, unquote(width), unquote(height)})

  @doc "Declares whether the native window can be resized by the user."
  defmacro resizable(value), do: quote(do: {:resizable, unquote(value)})

  @doc "Opts in to initial and changed appearance callbacks on the root view."
  defmacro appearance_events(value), do: quote(do: {:appearance_events, unquote(value)})

  @doc "Centers inside the visible display work area when restoration does not supply a frame."
  defmacro center_in_work_area(value), do: quote(do: {:center_in_work_area, unquote(value)})

  @doc "Opts in to initial and changed normal window geometry callbacks."
  defmacro geometry_events(value), do: quote(do: {:geometry_events, unquote(value)})

  @doc "Restores logical screen bounds if they still fit a connected display work area."
  defmacro restore_bounds(value), do: quote(do: {:restore_bounds, unquote(value)})

  @doc "Offsets restored bounds, wrapping at the display work area edge."
  defmacro cascade_offset(value), do: quote(do: {:cascade_offset, unquote(value)})

  @doc "Declares whether the window uses system or application-rendered content chrome."
  defmacro chrome(value), do: quote(do: {:chrome, unquote(value)})

  @doc "Binds an application command to a modified shortcut inside `window`."
  defmacro shortcut(id, keys), do: quote(do: {:command, unquote(id), unquote(keys)})

  @doc "Declares a root view inside `window`."
  defmacro root(module, assigns \\ []) do
    quote do
      {:root, unquote(module), unquote(assigns)}
    end
  end

  defp block_entries({:__block__, _meta, expressions}), do: expressions
  defp block_entries(expression), do: [expression]
end
