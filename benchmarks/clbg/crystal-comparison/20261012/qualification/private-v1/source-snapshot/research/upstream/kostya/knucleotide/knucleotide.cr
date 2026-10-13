def frecuency(seq, length)
  n = seq.size - length + 1
  table = Hash(String, Int32).new { 0 }
  (0...n).each do |f|
    table[seq.byte_slice(f, length)] += 1
  end
  {n, table}
end

def sort_by_freq(seq, length)
  n, table = frecuency(seq, length)
  table.to_a.sort { |a, b| b[1] <=> a[1] }.each do |v|
    puts "%s %.3f" % {v[0].upcase, ((v[1] * 100).to_f / n)}
  end
  puts
end

def find_seq(seq, s)
  n, table = frecuency(seq, s.size)
  puts "#{table[s].to_s}\t#{s.upcase}"
end

seq = IO::Memory.new
three = false

STDIN.each_line do |line|
  if line.starts_with?(">THREE")
    three = true
    next
  end
  seq << line.chomp if three
end
seq = seq.to_s

(1..2).each { |i| sort_by_freq(seq, i) }
%w(ggt ggta ggtatt ggtattttaatt ggtattttaatttatagt).each { |s| find_seq(seq, s) }
