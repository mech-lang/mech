defmodule Clbg.Main do
  alias Clbg.Support, as: S
  @from ~c"ACGTUMRWSYKVHDBNacgtumrwsykvhdbn"
  @to ~c"TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN"
  @pairs Enum.zip(@from, @to) |> Map.new()
  defp wrap([], _column, out), do: Enum.reverse(out)
  defp wrap([code | rest], 59, out), do: wrap(rest, 0, [10, code | out])
  defp wrap([code | rest], column, out), do: wrap(rest, column + 1, [code | out])
  defp record(header, parts) do
    sequence = parts |> Enum.reverse() |> IO.iodata_to_binary()
    reversed = sequence |> :binary.bin_to_list() |> Enum.reverse() |> Enum.map(fn code -> Map.fetch!(@pairs, code) end)
    suffix = if rem(length(reversed), 60) == 0, do: [], else: [10]
    [header, "\n", wrap(reversed, 0, []), suffix]
  end
  defp collect([], nil, _parts, out), do: Enum.reverse(out)
  defp collect([], header, parts, out), do: Enum.reverse([record(header, parts) | out])
  defp collect([<<">", _rest::binary>> = header | lines], old, parts, out) do
    out = if old == nil, do: out, else: [record(old, parts) | out]
    collect(lines, header, [], out)
  end
  defp collect([line | lines], header, parts, out) do
    parts = if header == nil or line == "" or :binary.at(line, 0) == ?;, do: parts, else: [line | parts]
    collect(lines, header, parts, out)
  end
  def main(_arguments), do: S.input() |> S.lines() |> collect(nil, [], []) |> S.write()
end
