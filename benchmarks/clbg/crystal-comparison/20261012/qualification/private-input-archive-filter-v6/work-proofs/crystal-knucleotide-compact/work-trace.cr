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

sequence = third_record
tables = LENGTHS.map do |width|
  counts = Hash(String, Int64).new(0_i64)
  Math.max(0, sequence.bytesize - width + 1).times do |start|
    counts[sequence.byte_slice(start, width)] += 1
  end
  counts
end
audit_trace_maps(tables, sequence)
2.times { |index| frequencies(tables[index]) }
QUERIES.each_with_index { |query, index| puts "#{tables[index + 2][query]}\t#{query}" }

def audit_trace_maps(tables, sequence)
  path = ENV["MECH_CRYSTAL_WORK_TRACE"]
  raise "Existing audit path" if File.exists?(path)
  File.open(path, "w") do |audit|
    audit.puts "TRACE_C1|knucleotide|compact|#{tables.size}|#{sequence.bytesize}"
    tables.each_with_index do |table, index|
      total = 0_i64
      table.each do |key, count|
        audit.puts "MAP|#{LENGTHS[index]}|#{key}|#{count}"
        total += count
      end
      audit.puts "TABLE|#{LENGTHS[index]}|#{table.size}|#{total}"
    end
    audit.puts "END_C1"
  end
end
