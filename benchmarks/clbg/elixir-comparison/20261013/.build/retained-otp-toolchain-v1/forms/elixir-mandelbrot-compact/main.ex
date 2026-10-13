defmodule Clbg.Main do
  import Bitwise
  alias Clbg.Support, as: S

  def inside(_cr, _ci, _zr, _zi, 50), do: true
  def inside(cr, ci, zr, zi, count) do
    next_r = zr * zr - zi * zi + cr
    next_i = 2.0 * zr * zi + ci
    if next_r * next_r + next_i * next_i > 4.0, do: false, else: inside(cr, ci, next_r, next_i, count + 1)
  end
  defp row(n, _y, n, byte, bits, out) do
    result = if bits == 0, do: out, else: [byte <<< (8 - bits) | out]
    result |> Enum.reverse() |> :erlang.list_to_binary()
  end
  defp row(x, y, n, byte, bits, out) do
    bit = if inside(2.0 * x / n - 1.5, 2.0 * y / n - 1.0, 0.0, 0.0, 0), do: 1, else: 0
    next = (byte <<< 1) ||| bit
    if bits == 7, do: row(x + 1, y, n, 0, 0, [next | out]), else: row(x + 1, y, n, next, bits + 1, out)
  end
  def main(arguments) do
    n = S.size(arguments, 200)
    if n == 0, do: raise(ArgumentError, "Positive size required")
    rows = for y <- S.indices(n), do: row(0, y, n, 0, 0, [])
    S.write(["P4\n#{n} #{n}\n", rows])
  end
end
