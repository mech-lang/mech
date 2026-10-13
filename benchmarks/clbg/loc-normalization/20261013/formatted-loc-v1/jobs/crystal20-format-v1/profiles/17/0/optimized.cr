COMPLEMENT = begin
  table = StaticArray(UInt8, 256).new { |index| index.to_u8 }
  letters = "ACGTUMRWSYKVHDBNacgtumrwsykvhdbn"
  values = "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN"
  letters.bytes.each_with_index { |value, index| table[value] = values.byte_at(index) }
  table
end

def write_complement(sequence : Array(UInt8))
  left = 0
  right = sequence.size - 1
  while left <= right
    a = sequence[left]
    b = sequence[right]
    sequence[left] = COMPLEMENT[b]
    sequence[right] = COMPLEMENT[a]
    left += 1
    right -= 1
  end
  output = IO::Memory.new(64 * 1024)
  offset = 0
  while offset < sequence.size
    count = Math.min(60, sequence.size - offset)
    output.write(Slice.new(sequence.to_unsafe, sequence.size)[offset, count])
    output.write_byte(10_u8)
    offset += count
    if output.bytesize >= 64 * 1024
      STDOUT.write(output.to_slice)
      output.clear
    end
  end
  STDOUT.write(output.to_slice)
end

input = STDIN.gets_to_end
bytes = input.to_slice
position = 0
header = false
sequence = [] of UInt8
while position < bytes.size
  first = position
  while position < bytes.size && bytes[position] != 10_u8 && bytes[position] != 13_u8
    position += 1
  end
  last = position
  if position < bytes.size
    separator = bytes[position]
    position += 1
    position += 1 if separator == 13_u8 && position < bytes.size && bytes[position] == 10_u8
  end
  next if first == last || bytes[first] == 59_u8
  if bytes[first] == 62_u8
    write_complement(sequence) if header
    sequence.clear
    STDOUT.write(bytes[first, last - first])
    STDOUT.write_byte(10_u8)
    header = true
  elsif header
    (first...last).each { |index| sequence << bytes[index] }
  end
end
write_complement(sequence) if header
