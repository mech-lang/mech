defmodule Clbg.Main do
  import Bitwise
  alias Clbg.Support, as: S

  def tree(0), do: {make_ref(), nil, nil}
  def tree(depth), do: {make_ref(), tree(depth - 1), tree(depth - 1)}
  def check({_identity, nil, nil}), do: 1
  def check({_identity, left, right}), do: 1 + check(left) + check(right)
  defp repeat(0, _depth, sum), do: sum
  defp repeat(n, depth, sum), do: repeat(n - 1, depth, sum + check(tree(depth)))
  defp depths(depth, maximum, long_lived, out) when depth > maximum do
    S.write([Enum.reverse(out), "long lived tree of depth #{maximum}\t check: #{check(long_lived)}\n"])
  end
  defp depths(depth, maximum, long_lived, out) do
    iterations = 1 <<< (maximum - depth + 4)
    line = "#{iterations}\t trees of depth #{depth}\t check: #{repeat(iterations, depth, 0)}\n"
    depths(depth + 2, maximum, long_lived, [line | out])
  end
  def main(arguments) do
    maximum = max(6, S.size(arguments, 10))
    stretch = "stretch tree of depth #{maximum + 1}\t check: #{check(tree(maximum + 1))}\n"
    long_lived = tree(maximum)
    depths(4, maximum, long_lived, [stretch])
  end
end
