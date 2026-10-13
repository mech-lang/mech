w = h = (ARGV[0]? || 100).to_i
STDOUT.print("P4\n#{w} #{h}\n")

bit_num = 0
byte_acc = 0_u8
ITER  =  50
LIMIT = 2.0

h.times do |y|
  w.times do |x|
    zr = zi = tr = ti = 0.0
    cr = (2.0 * x / w - 1.5)
    ci = (2.0 * y / h - 1.0)

    i = 0
    while (i < ITER) && (tr + ti <= LIMIT * LIMIT)
      zi = 2.0 * zr * zi + ci
      zr = tr - ti + cr
      tr = zr * zr
      ti = zi * zi
      i += 1
    end

    byte_acc <<= 1
    byte_acc |= 0x01 if tr + ti <= LIMIT * LIMIT
    bit_num += 1

    if bit_num == 8
      STDOUT.write_byte byte_acc
      byte_acc = 0_u8
      bit_num = 0
    elsif x == w - 1
      byte_acc <<= 8 - w % 8
      STDOUT.write_byte byte_acc
      byte_acc = 0_u8
      bit_num = 0
    end
  end
end
