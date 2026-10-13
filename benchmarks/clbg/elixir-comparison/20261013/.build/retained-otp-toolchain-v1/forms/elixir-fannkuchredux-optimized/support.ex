defmodule Clbg.Support do
  import Bitwise

  def size(arguments, fallback) do
    n = case arguments do
      [] -> fallback
      [text] -> String.to_integer(text)
      _ -> raise ArgumentError, "One size argument required"
    end
    if n < 0, do: raise(ArgumentError, "Negative size")
    n
  end
  def indices(0), do: []
  def indices(n) when n > 0, do: 0..(n - 1)
  defp configure do
    :ok = :io.setopts(:standard_io, binary: true, encoding: :latin1)
  end
  def input do
    configure()
    read([])
  end
  defp read(parts) do
    case IO.binread(:stdio, 65_536) do
      :eof -> parts |> Enum.reverse() |> IO.iodata_to_binary()
      {:error, reason} -> raise "Input error: #{inspect(reason)}"
      bytes when is_binary(bytes) -> read([bytes | parts])
    end
  end
  def write(data) do
    configure()
    case IO.binwrite(:stdio, data) do
      :ok -> :ok
      {:error, reason} -> raise "Output error: #{inspect(reason)}"
    end
  end
  def lines(bytes) do
    bytes |> :binary.replace("\r\n", "\n", [:global]) |> :binary.replace("\r", "\n", [:global]) |> :binary.split("\n", [:global])
  end
  defp power10(0), do: 1
  defp power10(n), do: 10 * power10(n - 1)
  def fixed(value, digits) when is_float(value) and is_integer(digits) and digits >= 0 and digits <= 18 do
    <<sign::1, exponent::11, fraction::52>> = <<value::float-64>>
    if exponent == 2047, do: raise(ArgumentError, "Nonfinite value")
    {mantissa, shift} = if exponent == 0, do: {fraction, -1074}, else: {(1 <<< 52) + fraction, exponent - 1075}
    scaled = mantissa * power10(digits)
    {numerator, denominator} = if shift >= 0, do: {scaled <<< shift, 1}, else: {scaled, 1 <<< (-shift)}
    quotient = div(numerator, denominator)
    remainder = rem(numerator, denominator)
    rounded = if 2 * remainder > denominator or (2 * remainder == denominator and rem(quotient, 2) == 1), do: quotient + 1, else: quotient
    text = rounded |> Integer.to_string() |> String.pad_leading(digits + 1, "0")
    decimal = if digits == 0, do: text, else: binary_part(text, 0, byte_size(text) - digits) <> "." <> binary_part(text, byte_size(text) - digits, digits)
    if sign == 1, do: "-" <> decimal, else: decimal
  end
end
