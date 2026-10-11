# The Computer Language Benchmarks Game
# http://benchmarksgame.alioth.debian.org
#
# Contributed by Aaron Tavistock

def process_segment(segment)
  (header, _, sequence) = segment.partition("\n")

  sequence.delete!("\n\r >")
  sequence.reverse!
  sequence.tr!(
    'wsatugcyrkmbdhvnATUGCYRKMBDHVN',
    'WSTAACGRYMKVHDBNTAACGRYMKVHDBN'
  )

  sequence_length = sequence.length
  results_size = sequence_length + sequence_length / 60 + header.length + 4
  results = String.new(">#{header}\n", capacity: results_size)

  idx = 0
  while idx < sequence_length
    results << sequence[idx,60]
    results << "\n"
    idx += 60
  end

  results.freeze
  results
end

def forking_worker(segment)
  reader, writer = IO.pipe

  pid = Process.fork do
    begin
      reader.close
      results = original_process_segment(segment)
      writer.write(results)
    ensure
      writer.close
    end
  end

  writer.close
  begin
    results = reader.read
  ensure
    reader.close
  end
  Process.waitpid(pid)

  results
end

if RUBY_PLATFORM != 'java'
  class << self
    alias_method :original_process_segment, :process_segment
    alias_method :process_segment, :forking_worker
  end
end

threads = []
STDIN.each_line('>').lazy.each do |segment|
  next if segment.length < 2
  threads << Thread.new do
    Thread.current[:output] = process_segment(segment)
  end
end
threads.each(&:join)

threads.each do |thread|
  puts thread[:output]
end
