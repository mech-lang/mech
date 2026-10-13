defmodule Clbg.Main do
  alias Clbg.Support, as: S

  def rotate(permutation, last) do
    first = elem(permutation, 0)
    moved = Enum.reduce(S.indices(last), permutation, fn i, result -> put_elem(result, i, elem(permutation, i + 1)) end)
    put_elem(moved, last, first)
  end
  def flips(permutation), do: permutation |> Tuple.to_list() |> List.to_tuple() |> flip_work(0)
  defp flip_work(permutation, total) do
    last = elem(permutation, 0)
    if last == 0 do
      total
    else
      reversed = Enum.reduce(0..last, permutation, fn i, result -> put_elem(result, i, elem(permutation, last - i)) end)
      flip_work(reversed, total + 1)
    end
  end
  defp ready(1, counts), do: counts
  defp ready(r, counts), do: ready(r - 1, put_elem(counts, r - 1, r))
  defp carry(permutation, counts, n, n), do: {permutation, counts, n}
  defp carry(permutation, counts, r, n) do
    permutation = rotate(permutation, r)
    counts = put_elem(counts, r, elem(counts, r) - 1)
    if elem(counts, r) > 0, do: {permutation, counts, r}, else: carry(permutation, counts, r + 1, n)
  end
  defp visit(permutation, counts, r, n, rank, checksum, maximum) do
    counts = ready(r, counts)
    amount = flips(permutation)
    checksum = checksum + if(rem(rank, 2) == 0, do: amount, else: -amount)
    maximum = max(maximum, amount)
    {permutation, counts, next} = carry(permutation, counts, 1, n)
    if next == n, do: S.write("#{checksum}\nPfannkuchen(#{n}) = #{maximum}\n"), else: visit(permutation, counts, next, n, rank + 1, checksum, maximum)
  end
  def main(arguments) do
    n = S.size(arguments, 7)
    if n == 0, do: raise(ArgumentError, "Positive size required")
    visit(S.indices(n) |> Enum.to_list() |> List.to_tuple(), List.duplicate(0, n) |> List.to_tuple(), n, n, 0, 0, 0)
  end
end
