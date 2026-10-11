# Full IUPAC complement/reversal and every record, using ordinary byte arrays.
import sys
table = bytearray(range(256))
for a,b in zip(b"ACBDGHKMNSRUTWVYacbdghkmnsrutwvy", b"TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"):
    table[a] = b
def show(header, sequence):
    sys.stdout.buffer.write(header.rstrip()+b"\n")
    for start in range(len(sequence)-1,-1,-60):
        end = max(-1,start-60)
        row = bytes(table[sequence[i]] for i in range(start,end,-1))
        sys.stdout.buffer.write(row+b"\n")
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
