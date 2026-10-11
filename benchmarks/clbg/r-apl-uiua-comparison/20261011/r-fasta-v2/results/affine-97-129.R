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
c <- coefficients(129,13,7,97)
seeds <- (c$multipliers*96+c$increments) %% 97
cat(format(seeds,scientific=FALSE,trim=TRUE),sep="\n")
