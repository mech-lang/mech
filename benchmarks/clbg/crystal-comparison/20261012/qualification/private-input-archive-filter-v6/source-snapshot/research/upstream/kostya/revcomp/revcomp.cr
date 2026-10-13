def revcomp(seq)
  seq = seq.reverse.tr("wsatugcyrkmbdhvnATUGCYRKMBDHVN", "WSTAACGRYMKVHDBNTAACGRYMKVHDBN")
  stringlen = seq.size - 1
  0.step(to: stringlen, by: 60) { |x| puts seq[x...x + 60] }
end

seq = IO::Memory.new

STDIN.each_line do |line|
  if line.starts_with? '>'
    if !seq.empty?
      revcomp(seq.to_s)
      seq.clear
    end
    puts(line)
  else
    seq << line.chomp
  end
end
revcomp(seq.to_s)
