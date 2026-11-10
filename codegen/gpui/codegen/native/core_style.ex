defmodule GPUI.Codegen.Native.CoreStyle do
  @moduledoc "Generates the renderer-independent style model owned by gpui_core."

  alias RustQ.Rust.AST, as: AST
  alias RustQ.Rust.AST.Builder, as: A

  @spec items() :: [AST.item()]
  def items do
    fields = Enum.map(GPUI.Schema.style_specs(), &field/1)

    [
      A.type_alias(
        :BoxShadowSpec,
        %AST.TypeTuple{items: Enum.map([:f32, :f32, :f32, :f32, :u32], &A.type_path/1)},
        vis: :pub
      ),
      %AST.Enum{
        name: :Length,
        variants: [
          %AST.EnumVariant{name: :Auto, tuple: []},
          %AST.EnumVariant{name: :Pixels, tuple: [A.type_path(:f32)]},
          %AST.EnumVariant{name: :Rems, tuple: [A.type_path(:f32)]},
          %AST.EnumVariant{name: :Fraction, tuple: [A.type_path(:f32)]}
        ],
        derive: [:Clone, :Copy, :Debug, :PartialEq],
        attrs: [],
        vis: :pub
      },
      %AST.Struct{
        name: :Style,
        fields: fields,
        derive: [:Clone, :Debug, :Default, :PartialEq],
        attrs: [],
        vis: :pub
      }
    ]
  end

  defp field(spec) do
    %AST.StructField{name: spec.field, type: field_type(spec.type), vis: :pub}
  end

  defp field_type(:box_shadows),
    do: option(A.type_path(:Vec, generics: [A.type_path(:BoxShadowSpec)]))

  defp field_type(:boolean), do: A.type_path(:bool)
  defp field_type({:atom_eq, _expected}), do: A.type_path(:bool)
  defp field_type(:group_name), do: option(A.type_path(:String))
  defp field_type(:atom_string), do: option(A.type_path(:String))

  defp field_type(:group_color),
    do: option(%AST.TypeTuple{items: [A.type_path(:String), A.type_path(:u32)]})

  defp field_type(:color), do: option(A.type_path(:u32))
  defp field_type(type) when type in [:number, :px, :radius], do: option(A.type_path(:f32))

  defp field_type(type) when type in [:length, :position_length, :flex_basis],
    do: option(A.type_path(:Length))

  defp option(type), do: A.type_path(:Option, generics: [type])
end
