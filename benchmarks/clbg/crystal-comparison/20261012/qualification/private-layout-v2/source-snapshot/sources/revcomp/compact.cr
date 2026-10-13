def reverse_complement(sequence : String)
  output = sequence.reverse.tr("ACGTUMRWSYKVHDBNacgtumrwsykvhdbn", "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN")
  0.step(to: output.bytesize - 1, by: 60) { |start| puts output.byte_slice(start, Math.min(60, output.bytesize - start)) }
end
header = false
sequence = String::Builder.new
STDIN.gets_to_end.split(/\r\n|\r|\n/).each do |line|
  next if line.starts_with?(";")
  if line.starts_with?(">")
    reverse_complement(sequence.to_s) if header
    sequence = String::Builder.new
    puts line
    header = true
  elsif header
    sequence << line
  end
end
reverse_complement(sequence.to_s) if header
