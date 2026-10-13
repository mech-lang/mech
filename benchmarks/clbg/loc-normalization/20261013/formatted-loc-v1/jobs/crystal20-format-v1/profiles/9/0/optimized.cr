alias Lane = StaticArray(Float64, 8)

def pixel_byte(cr : Lane, ci : Float64, valid : Int32) : UInt8
  zr = Lane.new(0.0)
  zi = Lane.new(0.0)
  rr = Lane.new(0.0)
  ii = Lane.new(0.0)
  10.times do
    5.times do
      8.times do |lane|
        zi[lane] = 2.0 * zr[lane] * zi[lane] + ci
        zr[lane] = rr[lane] - ii[lane] + cr[lane]
        rr[lane] = zr[lane] * zr[lane]
        ii[lane] = zi[lane] * zi[lane]
      end
    end
    escaped = true
    valid.times { |lane| escaped = false if rr[lane] + ii[lane] <= 4.0 }
    return 0_u8 if escaped
  end
  byte = 0_u8
  valid.times { |lane| byte |= (0x80_u8 >> lane) if rr[lane] + ii[lane] <= 4.0 }
  byte
end

def rows(n : Int32, first : Int32, last : Int32, done : Channel(Tuple(Int32, Bytes)))
  stride = (n + 7) // 8
  output = Bytes.new((last - first) * stride, 0_u8)
  scale = 2.0 / n
  (first...last).each do |y|
    stride.times do |column|
      valid = Math.min(8, n - column * 8)
      cr = Lane.new { |lane| (column * 8 + lane) * scale - 1.5 }
      output[(y - first) * stride + column] = pixel_byte(cr, y * scale - 1.0, valid)
    end
  end
  done.send({first, output})
end

n = (ARGV[0]? || "200").to_i
raise ArgumentError.new("Negative size") if n < 0
STDOUT.print("P4\n#{n} #{n}\n")
if n > 0
  Fiber::ExecutionContext.default.resize(8)
  workers = Math.min(8, n)
  done = Channel(Tuple(Int32, Bytes)).new(workers)
  workers.times do |worker|
    spawn rows(n, n * worker // workers, n * (worker + 1) // workers, done)
  end
  blocks = Array(Tuple(Int32, Bytes)).new(workers) { done.receive }
  blocks.sort_by! { |block| block[0] }
  blocks.each { |block| STDOUT.write(block[1]) }
end
