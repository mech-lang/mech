require "big"
n = (ARGV[0]? || "27").to_i
q = BigInt.new(1)
r = BigInt.new(0)
t = BigInt.new(1)
k = 1
printed = 0
line = String::Builder.new
while printed < n
  numerator3 = q * 3 + r
  numerator4 = q * 4 + r
  digit3 = numerator3 // t
  digit4 = numerator4 // t
  if digit3 == digit4
    line << digit3.to_s
    printed += 1
    if printed % 10 == 0 || printed == n
      line << " " * ((10 - printed % 10) % 10)
      puts "#{line.to_s}\t:#{printed}"
      line = String::Builder.new
    end
    r = (r - digit3 * t) * 10
    q *= 10
  else
    factor = 2 * k + 1
    r = (q * 2 + r) * factor
    q *= k
    t *= factor
    k += 1
  end
end
