# Generic modular affine-prefix coefficients, independent of the incoming seed.
coefficients <- function(size,multiplier,increment,modulus) {
  multipliers <- rep(multiplier,size)
  increments <- rep(increment,size)
  offset <- 1L
  while (offset < size) {
    suffix <- seq.int(offset+1L,size)
    prefix <- suffix-offset
    increments[suffix] <- (multipliers[suffix]*increments[prefix]+increments[suffix]) %% modulus
    multipliers[suffix] <- (multipliers[suffix]*multipliers[prefix]) %% modulus
    offset <- offset*2L
  }
  list(multipliers=multipliers,increments=increments)
}
# Generic affine-prefix scans materialize every RNG seed; no period/output cache.
c <- coefficients(257,17,9,65537)
seeds <- (c$multipliers*65536+c$increments) %% 65537
cat(format(seeds,scientific=FALSE,trim=TRUE),sep="\n")
