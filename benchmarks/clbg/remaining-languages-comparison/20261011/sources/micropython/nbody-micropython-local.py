# Locally authored full binary64 pairwise n-body workload.
import sys
from math import sqrt
mass = 4.0 * 3.141592653589793 * 3.141592653589793
year = 365.24
bodies = [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, mass],
    [4.84143144246472090, -1.16032004402742839, -0.103622044471123109, 0.00166007664274403694, 0.00769901118419740425, -0.0000690460016972063023, 0.000954791938424326609],
    [8.34336671824457987, 4.12479856412430479, -0.403523417114321381, -0.00276742510726862411, 0.00499852801234917238, 0.0000230417297573763929, 0.000285885980666130812],
    [12.8943695621391310, -15.1111514016986312, -0.223307578892655734, 0.00296460137564761618, 0.00237847173959480950, -0.0000296589568540253756, 0.0000436624404335156298],
    [15.3796971148509165, -25.9193146099879641, 0.179258772950371181, 0.00268067772490389322, 0.00162824170038242295, -0.0000951592254519715870, 0.0000515138902046611451]]
for b in bodies[1:]:
    b[6] *= mass
    for axis in range(3, 6):
        b[axis] *= year
for axis in range(3, 6):
    bodies[0][axis] = -sum(b[axis] * b[6] for b in bodies) / mass
pairs = [(a, b) for i, a in enumerate(bodies) for b in bodies[i+1:]]
def energy():
    kinetic = sum(0.5 * b[6] * sum(v*v for v in b[3:6]) for b in bodies)
    return kinetic - sum(a[6] * b[6] / sqrt(sum((a[i]-b[i])**2 for i in range(3))) for a, b in pairs)
print("%.9f" % energy())
for step in range(int(sys.argv[1])):
    for a, b in pairs:
        delta = [a[i]-b[i] for i in range(3)]
        square = sum(d*d for d in delta)
        magnitude = 0.01 / (square * sqrt(square))
        for axis in range(3):
            a[axis+3] -= delta[axis] * b[6] * magnitude
            b[axis+3] += delta[axis] * a[6] * magnitude
    for b in bodies:
        for axis in range(3):
            b[axis] += 0.01 * b[axis+3]
print("%.9f" % energy())
