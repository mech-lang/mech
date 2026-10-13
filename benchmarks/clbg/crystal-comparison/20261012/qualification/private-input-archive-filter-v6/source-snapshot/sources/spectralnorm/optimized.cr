def row_block(v : Array(Float64), result : Array(Float64), first : Int32, last : Int32, transpose : Bool, done : Channel(Nil))
  n = v.size
  (first...last).each do |i|
    denominator = transpose ? (i * (i + 1) // 2 + 1) : (i * (i + 1) // 2 + i + 1)
    a = 0.0
    b = 0.0
    c = 0.0
    d = 0.0
    j = 0
    while j + 3 < n
      a += v[j] / denominator
      denominator += i + j + (transpose ? 2 : 1)
      b += v[j + 1] / denominator
      denominator += i + j + (transpose ? 3 : 2)
      c += v[j + 2] / denominator
      denominator += i + j + (transpose ? 4 : 3)
      d += v[j + 3] / denominator
      denominator += i + j + (transpose ? 5 : 4)
      j += 4
    end
    sum = (a + b) + (c + d)
    while j < n
      sum += v[j] / denominator
      denominator += i + j + (transpose ? 2 : 1)
      j += 1
    end
    result[i] = sum
  end
  done.send(nil)
end
def apply(v : Array(Float64), transpose = false)
  n = v.size
  workers = Math.min(8, n)
  result = Array.new(n, 0.0)
  done = Channel(Nil).new(workers)
  workers.times { |worker| spawn row_block(v, result, n * worker // workers, n * (worker + 1) // workers, transpose, done) }
  workers.times { done.receive }
  result
end
n = (ARGV[0]? || "100").to_i
raise ArgumentError.new("Positive size required") if n <= 0
Fiber::ExecutionContext.default.resize(8)
u = Array.new(n, 1.0)
v = Array.new(n, 0.0)
10.times do
  v = apply(apply(u), true)
  u = apply(apply(v), true)
end
uv = 0.0
vv = 0.0
n.times do |i|
  uv += u[i] * v[i]
  vv += v[i] * v[i]
end
puts "%.9f" % Math.sqrt(uv / vv)
