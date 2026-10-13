require "big"
raise "Floor division" unless BigInt.new(-7) // BigInt.new(3) == -3
raise "Truncated division" unless BigInt.new(-7).tdiv(BigInt.new(3)) == -2
raise "Negative divisor" unless BigInt.new(7) // BigInt.new(-3) == -3
large = BigInt.new("92233720368547758081234567890")
old = large
changed = large * 17 + 9
raise "Immutable operand changed" unless old.to_s == "92233720368547758081234567890"
raise "Exact large division" unless (changed - 9) // 17 == old
decimal = "1234567890" * 1024
parsed = BigInt.new(decimal)
GC.collect
raise "Long constructor/lifetime" unless parsed.to_s == decimal && old == large
rejected = false
begin
  BigInt.new("not-an-integer")
rescue ArgumentError
  rejected = true
end
raise "Invalid constructor" unless rejected
raise "Nonoverlapping regex" unless "aaaaa".scan(/aa/).size == 2
raise "Regex replacements" unless "taNDHaD".gsub(/aND|Ha[DS]/, "<3>").gsub(/<[^>]*>/, "|") == "t||"
Fiber::ExecutionContext.default.resize(8)
raise "Compute capacity" unless Fiber::ExecutionContext.default.capacity == 8
done = Channel(Tuple(Int32, Hash(Int32, Int64))).new(8)
8.times do |index|
  spawn do
    counts = Hash(Int32, Int64).new(0_i64)
    1000.times { |value| counts[value % 17] += 1 }
    private_regex = Regex.new("a+")
    raise "Private regex" unless private_regex.match("aaa").not_nil![0] == "aaa"
    done.send({index, counts})
  end
end
received = Hash(Int32, Hash(Int32, Int64)).new
8.times do
  index, counts = done.receive
  raise "Duplicate worker" if received.has_key?(index)
  received[index] = counts
end
GC.collect
raise "Parent handoff" unless received.size == 8
received.each_value do |counts|
  raise "Complete growable map" unless counts.size == 17 && counts.values.sum(0_i64) == 1000
  17.times do |key|
    raise "Actual map value" unless counts[key] == 1000 // 17 + (key < 1000 % 17 ? 1 : 0)
  end
end
STDOUT.write("PRIMITIVES_OK\u0000\n".to_slice)
