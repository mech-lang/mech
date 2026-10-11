// Faithful Benchmarks Game workload; local Toit candidate.
import io.stdio
import bytes show Buffer

complements -> ByteArray:
  lookup := ByteArray 256: it
  symbols := "ABCDGHKMNRSTUVWY"
  values := "TVGHCDMKNYSAABWR"
  symbols.size.repeat:
    lookup[symbols[it]] = values[it]
    lookup[symbols[it] + 32] = values[it]
  return lookup

emit header/string? sequence/Buffer lookup/ByteArray:
  if not header: return
  print header
  data := sequence.bytes
  front := 0
  back := data.size - 1
  while front <= back:
    saved := lookup[data[front]]
    data[front] = lookup[data[back]]
    data[back] = saved
    front++
    back--
  output := Buffer
  for offset := 0; offset < data.size; offset += 60:
    output.write data offset (min data.size (offset + 60))
    output.write-byte '\n'
    if output.size >= 65536:
      stdio.stdout.write output.bytes
      output.clear
  stdio.stdout.write output.bytes

main args:
  lookup := complements
  header := null
  sequence := Buffer
  while line := stdio.stdin.read-line:
    if line.starts-with ">":
      emit header sequence lookup
      header = line
      sequence.clear
    else if not (line.starts-with ";"):
      sequence.write line
  emit header sequence lookup
