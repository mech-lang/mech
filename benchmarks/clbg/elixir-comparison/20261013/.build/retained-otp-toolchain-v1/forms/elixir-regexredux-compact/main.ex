defmodule Clbg.Main do
  alias Clbg.Support, as: S
  @variants ["agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]", "a[act]ggtaaa|tttacc[agt]t", "ag[act]gtaaa|tttac[agt]ct", "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct", "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct", "agggtaa[cgt]|[acg]ttaccct"]
  @replacements [{"tHa[Nt]", "<4>"}, {"aND|caN|Ha[DS]|WaS", "<3>"}, {"a[NSt]|BY", "<2>"}, {"<[^>]*>", "|"}, {"\\|[^|][^|]*\\|", "-"}]
  defp count(regex, text, position, total) do
    case Regex.run(regex, text, return: :index, capture: :first, offset: position) do
      nil -> total
      [{start, size}] ->
        if size == 0 and start == byte_size(text), do: total + 1, else: count(regex, text, start + max(size, 1), total + 1)
    end
  end
  defp replace(regex, text, replacement), do: replace(regex, text, replacement, 0, 0, [])
  defp replace(regex, text, replacement, position, cursor, out) do
    case Regex.run(regex, text, return: :index, capture: :first, offset: position) do
      nil -> [binary_part(text, cursor, byte_size(text) - cursor) | out] |> Enum.reverse() |> IO.iodata_to_binary()
      [{start, size}] ->
        out = [replacement, binary_part(text, cursor, start - cursor) | out]
        if size == 0 and start == byte_size(text) do
          out |> Enum.reverse() |> IO.iodata_to_binary()
        else
          replace(regex, text, replacement, start + max(size, 1), start + size, out)
        end
    end
  end
  def main(_arguments) do
    raw = S.input()
    dna = replace(Regex.compile!(">[^\n]*\n|\n"), raw, "")
    lines = for pattern <- @variants do
      amount = count(Regex.compile!(pattern), dna, 0, 0)
      "#{pattern} #{amount}\n"
    end
    changed = Enum.reduce(@replacements, dna, fn {pattern, replacement}, text -> replace(Regex.compile!(pattern), text, replacement) end)
    S.write([lines, "\n#{byte_size(raw)}\n#{byte_size(dna)}\n#{byte_size(changed)}\n"])
  end
end
