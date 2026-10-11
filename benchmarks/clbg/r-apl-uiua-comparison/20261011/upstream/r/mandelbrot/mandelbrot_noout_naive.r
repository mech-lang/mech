# ------------------------------------------------------------------
# The Computer Language Shootout
# http://shootout.alioth.debian.org/
#
# Contributed by Leo Osvald
# ------------------------------------------------------------------

lim <- 2
iter <- 50

mandelbrot_noout_naive <- function(args) {
    n = if (length(args)) as.integer(args[[1]]) else 200L
    n_mod8 = n %% 8L
    pads <- if (n_mod8) rep.int(0, 8L - n_mod8) else integer(0)
    p <- rep(as.integer(rep.int(2, 8) ^ (7:0)), length.out=n)

    cat("P4\n")
    cat(n, n, "\n")
    C <- matrix(0, n, n)
    for (y in 0:(n-1)) {
        C[, y] <- 2 * 0:(n-1) / n - 1.5 + 1i * (2 * y / n - 1)
    }

    m <- n
    Z <- 0                   # initialize Z to zero
    X <- array(0, c(m,m,20)) # initialize output 3D array
    for (k in 1:20) {        # loop with 20 iterations
        Z <- Z^2+C             # the central difference equation
          X[,,k] <- exp(-abs(Z)) # capture results
    }
}

execute <- function(n) {
    mandelbrot_noout_naive(n)
}
