package clbg.nbody
import clbg.Support
import kotlin.math.sqrt

val solar = 4.0 * Math.PI * Math.PI
val days = 365.24
fun planet(x: Double, y: Double, z: Double, vx: Double, vy: Double, vz: Double, mass: Double): DoubleArray = doubleArrayOf(x, y, z, vx * days, vy * days, vz * days, mass * solar)
fun energy(bodies: Array<DoubleArray>): Double {
    var total = 0.0
    var i = 0
    while (i < bodies.size) {
        val a = bodies[i]
        total += 0.5 * a[6] * (a[3] * a[3] + a[4] * a[4] + a[5] * a[5])
        var j = i + 1
        while (j < bodies.size) {
            val b = bodies[j]
            val dx = a[0] - b[0]; val dy = a[1] - b[1]; val dz = a[2] - b[2]
            total -= a[6] * b[6] / sqrt(dx * dx + dy * dy + dz * dz)
            j++
        }
        i++
    }
    return total
}
fun advance(bodies: Array<DoubleArray>) {
    var i = 0
    while (i < bodies.size - 1) {
        var j = i + 1
        while (j < bodies.size) {
            val a = bodies[i]; val b = bodies[j]
            val dx = a[0] - b[0]; val dy = a[1] - b[1]; val dz = a[2] - b[2]
            val squared = dx * dx + dy * dy + dz * dz
            val magnitude = 0.01 / (squared * sqrt(squared))
            a[3] -= dx * b[6] * magnitude; b[3] += dx * a[6] * magnitude
            a[4] -= dy * b[6] * magnitude; b[4] += dy * a[6] * magnitude
            a[5] -= dz * b[6] * magnitude; b[5] += dz * a[6] * magnitude
            j++
        }
        i++
    }
    for (body in bodies) { body[0] += 0.01 * body[3]; body[1] += 0.01 * body[4]; body[2] += 0.01 * body[5] }
}
fun main(args: Array<String>) {
    val bodies = arrayOf(
        doubleArrayOf(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, solar),
        planet(4.84143144246472090, -1.16032004402742839, -1.03622044471123109e-1, 1.66007664274403694e-3, 7.69901118419740425e-3, -6.90460016972063023e-5, 9.54791938424326609e-4),
        planet(8.34336671824457987, 4.12479856412430479, -4.03523417114321381e-1, -2.76742510726862411e-3, 4.99852801234917238e-3, 2.30417297573763929e-5, 2.85885980666130812e-4),
        planet(1.28943695621391310e1, -1.51111514016986312e1, -2.23307578892655734e-1, 2.96460137564761618e-3, 2.37847173959480950e-3, -2.96589568540237556e-5, 4.36624404335156298e-5),
        planet(1.53796971148509165e1, -2.59193146099879641e1, 1.79258772950371181e-1, 2.68067772490389322e-3, 1.62824170038242295e-3, -9.51592254519715870e-5, 5.15138902046611451e-5))
    var px = 0.0; var py = 0.0; var pz = 0.0
    for (body in bodies) { px += body[3] * body[6]; py += body[4] * body[6]; pz += body[5] * body[6] }
    bodies[0][3] = -px / solar; bodies[0][4] = -py / solar; bodies[0][5] = -pz / solar
    val before = Support.fixed(energy(bodies), 9)
    repeat(Support.size(args, 1000)) { advance(bodies) }
    Support.write(before + "\n" + Support.fixed(energy(bodies), 9) + "\n")
}
