defmodule Clbg.Main do
  alias Clbg.Support, as: S

  defp emit(_q, _r, _t, _k, count, wanted, _line, out) when count == wanted, do: S.write(Enum.reverse(out))
  defp emit(q, r, t, k, count, wanted, line, out) do
    three = div(3 * q + r, t)
    four = div(4 * q + r, t)
    if three == four do
      count = count + 1
      text = line <> Integer.to_string(three)
      out = if rem(count, 10) == 0 or count == wanted, do: [String.pad_trailing(text, 10, " ") <> "\t:#{count}\n" | out], else: out
      next_line = if rem(count, 10) == 0, do: "", else: text
      emit(10 * q, 10 * (r - t * three), t, k, count, wanted, next_line, out)
    else
      factor = 2 * k + 1
      emit(q * k, (r + 2 * q) * factor, t * factor, k + 1, count, wanted, line, out)
    end
  end
  def main(arguments), do: emit(1, 0, 1, 1, 0, S.size(arguments, 27), "", [])
end
