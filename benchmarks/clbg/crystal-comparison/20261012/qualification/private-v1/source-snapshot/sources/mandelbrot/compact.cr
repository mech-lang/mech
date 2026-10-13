n = (ARGV[0]? || "200").to_i
raise ArgumentError.new("Negative size") if n < 0
STDOUT.print("P4\n#{n} #{n}\n")
scale = 2.0 / n
n.times do |y|
  byte = 0_u8
  bits = 0
  n.times do |x|
    cr = x * scale - 1.5
    ci = y * scale - 1.0
    zr = 0.0
    zi = 0.0
    rr = 0.0
    ii = 0.0
    iterations = 0
    while iterations < 50 && rr + ii <= 4.0
      zi = 2.0 * zr * zi + ci
      zr = rr - ii + cr
      rr = zr * zr
      ii = zi * zi
      iterations += 1
    end
    byte = (byte << 1) | (rr + ii <= 4.0 ? 1_u8 : 0_u8)
    bits += 1
    if bits == 8 || x == n - 1
      STDOUT.write_byte(byte << (8 - bits))
      byte = 0_u8
      bits = 0
    end
  end
end
