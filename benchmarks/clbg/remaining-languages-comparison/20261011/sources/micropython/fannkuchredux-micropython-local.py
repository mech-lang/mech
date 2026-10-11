# Every factorial permutation, checksum and flip maximum; built-in list slices.
import sys
n = int(sys.argv[1])
permutation = list(range(n))
count = [0]*n
r = n
index = 0
checksum = 0
maximum = 0
while True:
    while r > 1:
        count[r-1] = r
        r -= 1
    work = permutation[:]
    flips = 0
    while work[0]:
        k = work[0]
        work[:k+1] = work[k::-1]
        flips += 1
    maximum = max(maximum, flips)
    checksum += flips if index%2 == 0 else -flips
    while r < n:
        first = permutation[0]
        permutation[:r] = permutation[1:r+1]
        permutation[r] = first
        count[r] -= 1
        if count[r]:
            break
        r += 1
    if r == n:
        break
    index += 1
print(checksum)
print("Pfannkuchen(%d) = %d" % (n, maximum))
