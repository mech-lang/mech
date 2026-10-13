defmodule Clbg.Main do
  alias Clbg.Support, as: S
  @alu "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
  @iub [{?a, 0.27}, {?c, 0.12}, {?g, 0.12}, {?t, 0.27}, {?B, 0.02}, {?D, 0.02}, {?H, 0.02}, {?K, 0.02}, {?M, 0.02}, {?N, 0.02}, {?R, 0.02}, {?S, 0.02}, {?V, 0.02}, {?W, 0.02}, {?Y, 0.02}]
  @homo [{?a, 0.3029549426680}, {?c, 0.1979883004921}, {?g, 0.1975473066391}, {?t, 0.3015094502008}]
  def cumulative(table) do
    {result, _sum} = Enum.map_reduce(table, 0.0, fn {code, probability}, sum -> {{code, sum + probability}, sum + probability} end)
    result
  end
  defp finish(column, line, out) do
    out = if column == 0, do: out, else: [:erlang.list_to_binary(Enum.reverse([10 | line])) | out]
    Enum.reverse(out)
  end
  defp repeated(0, _position, column, line, out), do: finish(column, line, out)
  defp repeated(count, position, column, line, out) do
    code = :binary.at(@alu, position)
    {column, line, out} = if column == 59, do: {0, [], [:erlang.list_to_binary(Enum.reverse([10, code | line])) | out]}, else: {column + 1, [code | line], out}
    repeated(count - 1, rem(position + 1, byte_size(@alu)), column, line, out)
  end
  defp choose([{code, _limit}], _draw), do: code
  defp choose([{code, limit} | rest], draw), do: if(draw < limit, do: code, else: choose(rest, draw))
  defp random(0, _table, seed, column, line, out), do: {finish(column, line, out), seed}
  defp random(count, table, seed, column, line, out) do
    seed = rem(seed * 3877 + 29573, 139968)
    code = choose(table, seed / 139968.0)
    {column, line, out} = if column == 59, do: {0, [], [:erlang.list_to_binary(Enum.reverse([10, code | line])) | out]}, else: {column + 1, [code | line], out}
    random(count - 1, table, seed, column, line, out)
  end
  def main(arguments) do
    n = S.size(arguments, 1000)
    one = repeated(2 * n, 0, 0, [], [])
    {two, seed} = random(3 * n, cumulative(@iub), 42, 0, [], [])
    {three, _seed} = random(5 * n, cumulative(@homo), seed, 0, [], [])
    S.write([">ONE Homo sapiens alu\n", one, ">TWO IUB ambiguity codes\n", two, ">THREE Homo sapiens frequency\n", three])
  end
end
