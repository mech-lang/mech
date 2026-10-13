defmodule Clbg.Main do
  alias Clbg.Support, as: S
  @queries ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]
  defp header_name(line) do
    line |> binary_part(1, byte_size(line) - 1) |> :binary.split([" ", "\t"]) |> hd()
  end
  defp sequence([], _selected, out), do: out |> Enum.reverse() |> IO.iodata_to_binary()
  defp sequence([<<">", _rest::binary>> = line | lines], selected, out) do
    if selected, do: sequence([], false, out), else: sequence(lines, header_name(line) == "THREE", out)
  end
  defp sequence([line | lines], selected, out) do
    if selected and line != "" and :binary.at(line, 0) != ?; do
      upper = for <<code <- line>>, into: <<>> do
        code = if code >= ?a and code <= ?z, do: code - 32, else: code
        if code not in [?A, ?C, ?G, ?T], do: raise(ArgumentError, "Unsupported nucleotide")
        <<code>>
      end
      sequence(lines, selected, [upper | out])
    else
      sequence(lines, selected, out)
    end
  end
  defp count(sequence, k, offset, counts) when offset + k > byte_size(sequence), do: counts
  defp count(sequence, k, offset, counts) do
    key = binary_part(sequence, offset, k)
    count(sequence, k, offset + 1, Map.update(counts, key, 1, fn value -> value + 1 end))
  end
  def main(_arguments) do
    dna = S.input() |> S.lines() |> sequence(false, [])
    maps = Map.new([1, 2, 3, 4, 6, 12, 18], fn k -> {k, count(dna, k, 0, %{})} end)
    frequencies = for k <- [1, 2] do
      ordered = maps[k] |> Enum.sort(fn {ka, va}, {kb, vb} -> va > vb or (va == vb and ka < kb) end)
      total = max(0, byte_size(dna) - k + 1)
      [Enum.map(ordered, fn {key, amount} -> [key, " ", S.fixed(100.0 * amount / total, 3), "\n"] end), "\n"]
    end
    queries = for query <- @queries, do: "#{Map.get(maps[byte_size(query)], query, 0)}\t#{query}\n"
    S.write([frequencies, queries])
  end
end
