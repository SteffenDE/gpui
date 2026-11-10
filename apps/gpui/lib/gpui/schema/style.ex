defmodule GPUI.Schema.Style do
  @moduledoc "Declarative schema for one renderer-independent style property."

  @enforce_keys [:name, :field, :type]
  defstruct [:name, :field, :type, :render]

  @type value_type ::
          :box_shadows
          | :boolean
          | :atom_string
          | :color
          | :group_name
          | :group_color
          | :number
          | :px
          | :length
          | :position_length
          | :flex_basis
          | :radius
          | {:atom_eq, atom()}

  @type render ::
          :box_shadows
          | :outer_ring
          | :flex_if_true
          | :truncate_if_true
          | {:if_true, atom()}
          | :group
          | :group_hover_background
          | :group_hover_visible
          | :group_hover_invisible
          | {:hover, atom(), :color}
          | {:enum_methods, [{String.t(), atom()}]}
          | {:enum_values, atom(), [{String.t(), [atom()]}]}
          | {:option_method, atom(),
             :color | :px | :length | :position_length | :flex_basis | :f32 | :string}
          | {:option_methods, [atom()], :position_length}

  @type t :: %__MODULE__{
          name: atom(),
          field: atom(),
          type: value_type(),
          render: render() | nil
        }
end
