defmodule GPUI.ImageTest do
  use ExUnit.Case, async: true

  @moduletag :native

  test "decodes encoded images into RGBA rasters" do
    assert {:ok,
            %GPUI.Raster{
              width: 2,
              height: 1,
              format: :rgba8,
              data: <<255, 0, 0, 255, 0, 255, 0, 255>>
            }} = GPUI.Image.decode(bmp_fixture())
  end

  test "decodes multicolor SVGs without tinting or premultiplying the public RGBA" do
    svg =
      ~s(<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="1" height="1" fill="red"/><rect x="1" width="1" height="1" fill="blue" fill-opacity="0.5"/></svg>)

    assert {:ok, %GPUI.Raster{width: 2, height: 1, data: <<255, 0, 0, 255, 0, 0, 255, 128>>}} =
             GPUI.Image.decode(svg)
  end

  test "SVG dimensions obey the raster decoder limits" do
    assert {:error, :invalid_image} =
             GPUI.Image.decode(
               ~s(<svg xmlns="http://www.w3.org/2000/svg" width="16385" height="1"/>)
             )
  end

  test "rejects malformed encoded images" do
    assert {:error, :invalid_image} = GPUI.Image.decode("not an image")
  end

  defp bmp_fixture do
    <<
      "BM",
      62::little-32,
      0::little-16,
      0::little-16,
      54::little-32,
      40::little-32,
      2::little-signed-32,
      1::little-signed-32,
      1::little-16,
      24::little-16,
      0::little-32,
      8::little-32,
      2_835::little-signed-32,
      2_835::little-signed-32,
      0::little-32,
      0::little-32,
      0,
      0,
      255,
      0,
      255,
      0,
      0,
      0
    >>
  end
end
