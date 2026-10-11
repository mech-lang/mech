# ------------------------------------------------------------------
# The Computer Language Shootout
# http://shootout.alioth.debian.org/
#
# Contributed by Leo Osvald
# ------------------------------------------------------------------

pi <- 3.141592653589793
solar_mass <- 4 * pi * pi
days_per_year <- 365.24
n_bodies <- 5

body_x <- c(
    0, # sun
    4.84143144246472090e+00, # jupiter
    8.34336671824457987e+00, # saturn
    1.28943695621391310e+01, # uranus
    1.53796971148509165e+01 # neptune
)
body_y <- c(
    0, # sun
    -1.16032004402742839e+00, # jupiter
    4.12479856412430479e+00, # saturn
    -1.51111514016986312e+01, # uranus
    -2.59193146099879641e+01 # neptune
)
body_z <- c(
    0, # sun
    -1.03622044471123109e-01, # jupiter
    -4.03523417114321381e-01, # saturn
    -2.23307578892655734e-01, # uranus
    1.79258772950371181e-01 # neptune
)

body_vx <- c(
    0, # sun
    1.66007664274403694e-03 * days_per_year, # jupiter
    -2.76742510726862411e-03 * days_per_year, # saturn
    2.96460137564761618e-03 * days_per_year, # uranus
    2.68067772490389322e-03 * days_per_year # neptune
)
body_vy <- c(
    0, # sun
    7.69901118419740425e-03 * days_per_year, # jupiter
    4.99852801234917238e-03 * days_per_year, # saturn
    2.37847173959480950e-03 * days_per_year, # uranus
    1.62824170038242295e-03 * days_per_year # neptune
)
body_vz <- c(
    0, # sun
    -6.90460016972063023e-05 * days_per_year, # jupiter
    2.30417297573763929e-05 * days_per_year, # saturn
    -2.96589568540237556e-05 * days_per_year, # uranus
    -9.51592254519715870e-05 * days_per_year # neptune
)

body_mass <- c(
    solar_mass, # sun
    9.54791938424326609e-04 * solar_mass, # jupiter
    2.85885980666130812e-04 * solar_mass, # saturn
    4.36624404335156298e-05 * solar_mass, # uranus
    5.15138902046611451e-05 * solar_mass # neptune
)

offset_momentum <- function() {
    body_vx[[1]] <<- -sum(body_vx * body_mass) / solar_mass
    body_vy[[1]] <<- -sum(body_vy * body_mass) / solar_mass
    body_vz[[1]] <<- -sum(body_vz * body_mass) / solar_mass
}

advance <- function(dt) {
    dxx <- matrix(0, n_bodies, n_bodies)
    dyy <- matrix(0, n_bodies, n_bodies)
    dzz <- matrix(0, n_bodies, n_bodies)
    for (i in 1:n_bodies) {
        for (j in 1:n_bodies) {
            dxx[[i, j]] <- body_x[[i]] - body_x[[j]]
            dyy[[i, j]] <- body_y[[i]] - body_y[[j]]
            dzz[[i, j]] <- body_z[[i]] - body_z[[j]]
        }
    }

    for (i in 1:(n_bodies - 1)) {
        j_from <- min(i + 1, n_bodies)
        for (j in j_from:n_bodies) {
            dx <- body_x[[i]] - body_x[[j]]
            dy <- body_y[[i]] - body_y[[j]]
            dz <- body_z[[i]] - body_z[[j]]
            distance <- sqrt(dx * dx + dy * dy + dz * dz)
            mag <- dt / (distance * distance * distance)
            body_vx[[i]] <<- body_vx[[i]] - dx * body_mass[[j]] * mag
            body_vy[[i]] <<- body_vy[[i]] - dy * body_mass[[j]] * mag
            body_vz[[i]] <<- body_vz[[i]] - dz * body_mass[[j]] * mag
            body_vx[[j]] <<- body_vx[[j]] + dx * body_mass[[i]] * mag
            body_vy[[j]] <<- body_vy[[j]] + dy * body_mass[[i]] * mag
            body_vz[[j]] <<- body_vz[[j]] + dz * body_mass[[i]] * mag
        }
    }

    for (i in 1:n_bodies) {
        body_x[[i]] <<- body_x[[i]] + dt * body_vx[[i]]
        body_y[[i]] <<- body_y[[i]] + dt * body_vy[[i]]
        body_z[[i]] <<- body_z[[i]] + dt * body_vz[[i]]
    }
}

energy <- function() {
    # this is only called twice, so the way of implementing it is not important
    dxx <- outer(body_x, body_x, "-")
    dyy <- outer(body_y, body_y, "-")
    dzz <- outer(body_z, body_z, "-")

    distance <- sqrt(dxx * dxx + dyy * dyy + dzz * dzz)
    q <- (body_mass %o% body_mass) / distance
    return(sum(0.5 * body_mass *
               (body_vx * body_vx + body_vy * body_vy + body_vz * body_vz)) -
           sum(q[upper.tri(q)]))
}

nbody_naive <- function(args) {
    n = if (length(args)) as.integer(args[[1]]) else 1000L
    options(digits=9)
    offset_momentum()
    cat(energy(), "\n")
    for (i in 1:n)
        advance(0.01)
    cat(energy(), "\n")
}

execute <- function(n) {
    nbody_naive(n)
}
