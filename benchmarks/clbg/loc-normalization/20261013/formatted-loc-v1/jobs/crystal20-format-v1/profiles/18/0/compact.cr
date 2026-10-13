def apply(v, transpose = false)
  Array.new(v.size) do |i|
    sum = 0.0
    v.each_with_index do |value, j|
      row = transpose ? j : i
      column = transpose ? i : j
      sum += value / ((row + column) * (row + column + 1) // 2 + row + 1)
    end
    sum
  end
end

n = (ARGV[0]? || "100").to_i
raise ArgumentError.new("Positive size required") if n <= 0
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
