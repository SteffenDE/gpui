defmodule GPUI.TailwindCacheTest do
  use ExUnit.Case, async: false

  test "class normalization survives concurrent short-lived render processes" do
    # Each rendering process may be the first caller and hence own the optional
    # cache. Its exit must not make normalization fail in another window.
    1..2_000
    |> Task.async_stream(
      fn width ->
        class = "w-[#{width}px] h-9 text-[#d7dbe1]"

        for _ <- 1..4 do
          assert %{unknown: [], style: style} = GPUI.Tailwind.normalize(class)
          assert style[:width] == {:px, width * 1.0}
          assert style[:height] == {:px, 36.0}
          assert style[:color] == {:rgb, 0xD7DBE1}
        end
      end,
      max_concurrency: 32,
      ordered: false
    )
    |> Enum.each(fn result -> assert {:ok, _} = result end)
  end
end
