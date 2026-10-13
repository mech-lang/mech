require "big"
n = (ARGV[0]? || "27").to_i
upper = BigInt.new(4)
lower = BigInt.new(3)
denominator = BigInt.new(1)
k = 1
printed = 0
line = String::Builder.new
while printed < n
  digit4 = upper // denominator
  digit3 = lower // denominator
  if digit3 == digit4
    line << digit3.to_s
    printed += 1
    if printed % 10 == 0 || printed == n
      line << " " * ((10 - printed % 10) % 10)
      puts "#{line.to_s}\t:#{printed}"
      line = String::Builder.new
    end
    subtract = digit3 * denominator * 10
    upper = upper * 10 - subtract
    lower = lower * 10 - subtract
  else
    next_upper = upper * (2 * k - 1) + lower * 2
    next_lower = upper * (k - 1) + lower * (k + 2)
    upper = next_upper
    lower = next_lower
    denominator *= 2 * k + 1
    k += 1
  end
end
