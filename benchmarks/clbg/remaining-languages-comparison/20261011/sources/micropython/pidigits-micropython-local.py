# Unbounded integer Gibbons LFT; both extract(3) and extract(4) each attempt.
import sys
limit = int(sys.argv[1])
q = 1
r = 0
t = 1
k = 1
count = 0
line = ""
while count < limit:
    a = (3*q+r)//t
    b = (4*q+r)//t
    if a == b:
        line += str(a)
        count += 1
        if count%10 == 0 or count == limit:
            print(line+" "*(10-len(line))+"\t:"+str(count))
            line = ""
        r = 10*(r-a*t)
        q *= 10
    else:
        r = (2*q+r)*(2*k+1)
        q *= k
        t *= 2*k+1
        k += 1
