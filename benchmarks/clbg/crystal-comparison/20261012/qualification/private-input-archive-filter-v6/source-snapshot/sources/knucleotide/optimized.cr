LENGTHS = [1, 2, 3, 4, 6, 12, 18]
QUERIES = ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]
def third_record
  selected = false
  sequence = String::Builder.new
  STDIN.gets_to_end.split(/\r\n|\r|\n/).each do |line|
    next if line.starts_with?(";")
    if line.starts_with?(">")
      break if selected
      selected = line == ">THREE" || line.starts_with?(">THREE ") || line.starts_with?(">THREE\t")
    elsif selected
      sequence << line.upcase
    end
  end
  sequence.to_s
end
def frequencies(table : Hash(String, Int64))
  total = table.values.sum(0_i64)
  rows = table.to_a.sort do |left, right|
    left[1] == right[1] ? left[0] <=> right[0] : right[1] <=> left[1]
  end
  rows.each { |row| puts "#{row[0]} #{"%.3f" % (100.0 * row[1] / total)}" }
  puts ""
end

def code(value : UInt8) : UInt64
  case value
  when 65_u8 then 0_u64
  when 67_u8 then 1_u64
  when 71_u8 then 2_u64
  when 84_u8 then 3_u64
  else raise ArgumentError.new("Invalid nucleotide")
  end
end
def decode(key : UInt64, width : Int32) : String
  output = Bytes.new(width)
  remaining = key
  (width - 1).downto(0) do |index|
    output[index] = "ACGT".byte_at((remaining & 3_u64).to_i)
    remaining >>= 2
  end
  String.new(output)
end
def histogram(sequence : String, width : Int32, index : Int32, done : Channel(Tuple(Int32, Hash(UInt64, Int64))))
  counts = Hash(UInt64, Int64).new(0_i64)
  mask = (1_u64 << (2 * width)) - 1_u64
  key = 0_u64
  sequence.to_slice.each_with_index do |value, position|
    key = ((key << 2) | code(value)) & mask
    counts[key] += 1 if position + 1 >= width
  end
  done.send({index, counts})
end
sequence = third_record
Fiber::ExecutionContext.default.resize(8)
done = Channel(Tuple(Int32, Hash(UInt64, Int64))).new(7)
LENGTHS.each_with_index { |width, index| spawn histogram(sequence, width, index, done) }
tables = Array.new(7) { Hash(UInt64, Int64).new(0_i64) }
7.times do
  record = done.receive
  tables[record[0]] = record[1]
end
2.times do |index|
  rows = Hash(String, Int64).new(0_i64)
  tables[index].each { |key, value| rows[decode(key, LENGTHS[index])] = value }
  frequencies(rows)
end
QUERIES.each_with_index do |query, index|
  key = 0_u64
  query.to_slice.each { |value| key = (key << 2) | code(value) }
  puts "#{tables[index + 2][key]}\t#{query}"
end
