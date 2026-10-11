# Scalar binary64 pixels, strict radius test, complete padded PBM rows.
import sys
n = int(sys.argv[1])
write = sys.stdout.buffer.write
write(("P4\n%d %d\n" % (n, n)).encode())
real = [2.0*x/n - 1.5 for x in range(n)]
for y in range(n):
    ci = 2.0*y/n - 1.0
    row = bytearray((n+7)//8)
    for x in range(n):
        zr = 0.0
        zi = 0.0
        inside = 1
        for iteration in range(50):
            next_real = zr*zr - zi*zi + real[x]
            zi = 2.0*zr*zi + ci
            zr = next_real
            if zr*zr + zi*zi > 4.0:
                inside = 0
                break
        row[x//8] |= inside << (7-x%8)
    write(row)
