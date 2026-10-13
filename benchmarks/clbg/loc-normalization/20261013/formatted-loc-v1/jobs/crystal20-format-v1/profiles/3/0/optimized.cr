def rank_range(n : Int32, first : Int64, last : Int64) : Tuple(Int64, Int32)
  factorial = StaticArray(Int64, 21).new(1_i64)
  (1..n).each { |width| factorial[width] = factorial[width - 1] * width }
  permutation = StaticArray(Int32, 32).new { |i| i }
  rotations = StaticArray(Int32, 32).new(0)
  n.downto(2) do |width|
    shift = ((first // factorial[width - 1]) % width).to_i
    rotations[width - 1] = shift
    shift.times do
      initial = permutation[0]
      (width - 1).times { |j| permutation[j] = permutation[j + 1] }
      permutation[width - 1] = initial
    end
  end
  checksum = 0_i64
  maximum = 0
  (first...last).each do |rank|
    work = permutation
    flips = 0
    while work[0] != 0
      left = 0
      right = work[0]
      while left < right
        temporary = work[left]
        work[left] = work[right]
        work[right] = temporary
        left += 1
        right -= 1
      end
      flips += 1
    end
    checksum += rank.even? ? flips : -flips
    maximum = Math.max(maximum, flips)
    (2..n).each do |width|
      initial = permutation[0]
      (width - 1).times { |j| permutation[j] = permutation[j + 1] }
      permutation[width - 1] = initial
      rotations[width - 1] += 1
      break if rotations[width - 1] < width
      rotations[width - 1] = 0
    end
  end
  {checksum, maximum}
end

def range_job(n : Int32, first : Int64, last : Int64, done : Channel(Tuple(Int64, Int32)))
  done.send(rank_range(n, first, last))
end

n = (ARGV[0]? || "10").to_i
raise ArgumentError.new("Size must be 1..20") unless n >= 1 && n <= 20
total = 1_i64
(1..n).each { |width| total *= width }
workers = Math.min(8_i64, total).to_i
done = Channel(Tuple(Int64, Int32)).new(workers)
Fiber::ExecutionContext.default.resize(8)
workers.times do |worker|
  first = (total // workers) * worker + Math.min(worker.to_i64, total % workers)
  last = (total // workers) * (worker + 1) + Math.min((worker + 1).to_i64, total % workers)
  spawn range_job(n, first, last, done)
end
checksum = 0_i64
maximum = 0
workers.times do
  result = done.receive
  checksum += result[0]
  maximum = Math.max(maximum, result[1])
end
puts checksum
puts "Pfannkuchen(#{n}) = #{maximum}"
