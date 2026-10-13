defmodule Clbg.Main do
  alias Clbg.Support, as: S

  def a(i, j), do: 1.0 / (div((i + j) * (i + j + 1), 2) + i + 1)
  def multiply(input, transpose) do
    n = tuple_size(input)
    for i <- S.indices(n) do
      Enum.reduce(S.indices(n), 0.0, fn j, sum ->
        entry = if transpose, do: a(j, i), else: a(i, j)
        sum + entry * elem(input, j)
      end)
    end |> List.to_tuple()
  end
  def ata(input), do: input |> multiply(false) |> multiply(true)
  defp rounds(0, u, v), do: {u, v}
  defp rounds(n, u, _v) do
    v = ata(u)
    rounds(n - 1, ata(v), v)
  end
  def main(arguments) do
    n = S.size(arguments, 100)
    if n == 0, do: raise(ArgumentError, "Positive size required")
    {u, v} = rounds(10, List.duplicate(1.0, n) |> List.to_tuple(), {})
    {uv, vv} = Enum.reduce(S.indices(n), {0.0, 0.0}, fn i, {uv, vv} -> {uv + elem(u, i) * elem(v, i), vv + elem(v, i) * elem(v, i)} end)
    S.write([S.fixed(:math.sqrt(uv / vv), 9), "\n"])
  end
end
