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

body_r <- matrix(c(
    c(  # sun
	0,
	0,
	0),
    c(  # jupiter
	4.84143144246472090e+00,
	-1.16032004402742839e+00,
	-1.03622044471123109e-01),
    c(  # saturn
	8.34336671824457987e+00,
	4.12479856412430479e+00,
	-4.03523417114321381e-01),
    c(  # uranus
	1.28943695621391310e+01,
	-1.51111514016986312e+01,
	-2.23307578892655734e-01),
    c(  # neptune
	1.53796971148509165e+01,
	-2.59193146099879641e+01,
	1.79258772950371181e-01)
), 3)

body_v <- matrix(c(
    c(  #sun
	0,
	0,
	0),
    c(  # jupiter
	1.66007664274403694e-03 * days_per_year,
	7.69901118419740425e-03 * days_per_year,
	-6.90460016972063023e-05 * days_per_year),
    c(  # saturn
	-2.76742510726862411e-03 * days_per_year,
	4.99852801234917238e-03 * days_per_year,
	2.30417297573763929e-05 * days_per_year),
    c(  # uranus
	2.96460137564761618e-03 * days_per_year,
	2.37847173959480950e-03 * days_per_year,
	-2.96589568540237556e-05 * days_per_year),
    c(  # neptune
	2.68067772490389322e-03 * days_per_year,
	1.62824170038242295e-03 * days_per_year,
	-9.51592254519715870e-05 * days_per_year)
), 3)

body_mass <- c(
    solar_mass, # sun
    9.54791938424326609e-04 * solar_mass, # jupiter
    2.85885980666130812e-04 * solar_mass, # saturn
    4.36624404335156298e-05 * solar_mass, # uranus
    5.15138902046611451e-05 * solar_mass # neptune
)

offset_momentum <- function() {
    body_v[, 1] <<- -(body_v %*% body_mass) / solar_mass
}

advance <- function(dt) {
    drr <- array(dim=c(n_bodies, n_bodies, 3))
    for (i in 1:n_bodies) {
        for (j in 1:n_bodies) {
            drr[i, j,] <- body_r[,i] - body_r[,j]
        }
    }
    for (i in 1:(n_bodies - 1)) {
        j_from <- min(i + 1, n_bodies)
        for (j in j_from:n_bodies) {
            dr <- body_r[, i] - body_r[, j]
            distance <- sqrt(sum(dr * dr))
            mag <- dt / (distance * distance * distance)
            body_v[, i] <<- body_v[, i] - dr * body_mass[[j]] * mag
            body_v[, j] <<- body_v[, j] + dr * body_mass[[i]] * mag
        }
    }
    for (i in 1:n_bodies)
        body_r[, i] <<- body_r[, i] + dt * body_v[, i]
}

energy <- function() {
    # this is only called twice, so the way of implementing it is not important
    drr <- array(dim=c(n_bodies, n_bodies, 3))
    for (i in 1:n_bodies) {
        for (j in 1:n_bodies) {
            drr[i, j,] <- body_r[,i] - body_r[,j]
        }
    }
    distance <- sqrt(t(colSums(aperm(drr * drr))))
    q <- (body_mass %o% body_mass) / distance
    return(sum(0.5 * body_mass * colSums(body_v * body_v)) -
           sum(q[upper.tri(q)]))
}

nbody_naive_2 <- function(args) {
    n = if (length(args)) as.integer(args[[1]]) else 1000L
    options(digits=9)
    offset_momentum()
    cat(energy(), "\n")
    for (i in 1:n)
        advance(0.01)
    cat(energy(), "\n")
}

execute <- function(n) {
    nbody_naive_2(n)
}
