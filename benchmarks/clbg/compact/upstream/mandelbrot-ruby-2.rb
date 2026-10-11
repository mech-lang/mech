# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
#  contributed by Karl von Laudermann
#  modified by Jeremy Echols
#  modified by Detlef Reichl
#  modified by Joseph LaFata

PAD = "\\\\__MARSHAL_RECORD_SEPARATOR__//" # silly, but works

class Worker
  
  attr_reader :reader
  
  def initialize(enum, index, total, &block)
    @enum             = enum
    @index            = index
    @total            = total
    @reader, @writer  = IO.pipe
    

      @p = Process.fork do
        @reader.close
        self.execute(&block)
        @writer.close
      end
      
      @writer.close
  end
  
  def execute(&block)
    (0 ... @enum.size).step(@total) do |bi|
      idx = bi + @index
      if item = @enum[idx]
        res = yield(item)
        @writer.write(Marshal.dump([idx, res]) + PAD)
      end
    end
    
    @writer.write(Marshal.dump(:end) + PAD)
  end
end

def parallel_map(enum, worker_count = 8, &block)
  count = [enum.size, worker_count].min
  
  Array.new(enum.size).tap do |res|  
    workers = (0 ... count).map do |idx|
      Worker.new(enum, idx, count, &block)
    end
  
    ios = workers.map { |w| w.reader }

    while ios.size > 0 do
      sr, sw, se = IO.select(ios, nil, nil, 0.01)

      if sr
        sr.each do |io|
          buf = ""
          
          while sbuf = io.readpartial(4096)
            buf += sbuf
            break if sbuf.size < 4096
          end
          
          msgs = buf.split(PAD)
          
          msgs.each do |msg|
            m = Marshal.load(msg)
            if m == :end
              ios.delete(io)
            else
              idx, content = m
              res[idx] = content
            end
          end
        end
      end      
    end
    
    Process.waitall
  end
end

$size = (ARGV[0] || 100).to_i
csize = $size - 1

puts "P4"
puts "#{$size} #{$size}"

set = (0 ... $size).to_a

results = parallel_map(set, 8) do |y|
  res = ""
  
  byte_acc = 0
  bit_num  = 0
  
  ci = (2.0 * y / $size) - 1.0

  $size.times do |x|
    zrzr = zr = 0.0
    zizi = zi = 0.0
    cr = (2.0 * x / $size) - 1.5
    escape = 0b1
  
    50.times do
      tr = zrzr - zizi + cr
      ti = 2.0 * zr * zi + ci
      zr = tr
      zi = ti
      # preserve recalculation
      zrzr = zr * zr
      zizi = zi * zi
      if zrzr + zizi > 4.0
        escape = 0b0
        break
      end
    end
  
    byte_acc = (byte_acc << 1) | escape
    bit_num  += 1
    
    if (bit_num == 8)
      res += byte_acc.chr
      byte_acc = 0
      bit_num = 0
    elsif (x == csize)
      byte_acc <<= (8 - bit_num)
      res += byte_acc.chr
      byte_acc = 0
      bit_num = 0
    end
  end

  res
end

print results.join
