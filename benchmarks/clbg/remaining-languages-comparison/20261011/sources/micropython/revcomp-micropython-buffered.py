# Ordinary mutable reversal, generic complement table, buffered wrapped output.
import sys
table = bytearray(range(256))
for a,b in zip(b"ACBDGHKMNSRUTWVYacbdghkmnsrutwvy", b"TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"):
    table[a] = b
write = sys.stdout.buffer.write
def show(header, sequence):
    write(header.rstrip()+b"\n")
    low = 0
    high = len(sequence)-1
    while low <= high:
        complement = table[sequence[low]]
        sequence[low] = table[sequence[high]]
        sequence[high] = complement
        low += 1
        high -= 1
    pending = bytearray()
    for start in range(0,len(sequence),60):
        pending.extend(sequence[start:start+60])
        pending.append(10)
        if len(pending) >= 65536:
            write(pending)
            pending = bytearray()
    write(pending)
header = None
sequence = bytearray()
for line in sys.stdin.buffer:
    if line.startswith(b">"):
        if header is not None:
            show(header,sequence)
        header = line
        sequence = bytearray()
    else:
        sequence.extend(line.strip())
if header is not None:
    show(header,sequence)
