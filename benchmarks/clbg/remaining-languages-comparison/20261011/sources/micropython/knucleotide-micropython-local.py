# Complete seven growable hash maps, collision-free full 36-bit rolling keys.
import sys
lines = []
for line in sys.stdin:
    if line.startswith(">THREE"):
        break
for line in sys.stdin:
    if line.startswith(">"):
        break
    lines.append(line.strip().upper())
dna = "".join(lines)
codes = bytes((ord(c)&6)>>1 for c in dna)
def packed(text):
    key = 0
    for c in text:
        key = (key<<2) | ((ord(c)&6)>>1)
    return key
def decode(key, width):
    return "".join("ACTG"[(key>>(2*i))&3] for i in range(width-1,-1,-1))
def counts(width):
    result = {}
    mask = (1<<(2*width))-1
    key = 0
    for i, code in enumerate(codes):
        key = ((key<<2)|code)&mask
        if i >= width-1:
            result[key] = result.get(key,0)+1
    return result
for width in [1,2]:
    table = counts(width)
    for key, count in sorted(table.items(), key=lambda p: (-p[1],decode(p[0],width))):
        print("%s %.3f" % (decode(key,width),100.0*count/(len(codes)-width+1)))
    print()
for query in ["GGT","GGTA","GGTATT","GGTATTTTAATT","GGTATTTTAATTTATAGT"]:
    print("%d\t%s" % (counts(len(query)).get(packed(query),0),query))
