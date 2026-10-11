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
run <- function(n) {
  seed <- 42L
  output <- file("/dev/stdout","wb",raw=TRUE)
  emit <- function(x) writeBin(x,output)
  header <- function(x) emit(charToRaw(paste0(x,"\n")))
  block <- 60L*4096L
  coefs <- coefficients(block,3877.0,29573.0,139968.0)
  wrap <- function(x) {
    length <- length(x)
    bytes <- rep(as.raw(10L),length+ceiling(length/60L))
    bytes[seq_len(length)+(seq_len(length)-1L) %/% 60L] <- x
    emit(bytes)
  }
  generate <- function(letters,probabilities,n) {
    cdf <- cumsum(probabilities)
    letters <- charToRaw(letters)
    for (start in seq.int(0,n-1,by=block)) {
      size <- min(block,n-start)
      index <- seq_len(size)
      seeds <- (coefs$multipliers[index]*seed+coefs$increments[index]) %% 139968
      random <- seeds/139968
      seed <- seeds[size]
      wrap(letters[findInterval(random,cdf)+1L])
    }
    seed
  }
  alu <- charToRaw("GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA")
  header(">ONE Homo sapiens alu")
  for (start in seq.int(0,2*n-1,by=block)) wrap(alu[(start+seq_len(min(block,2*n-start))-1) %% length(alu)+1L])
  header(">TWO IUB ambiguity codes")
  seed <- generate("acgtBDHKMNRSVWY",c(0.27,0.12,0.12,0.27,rep(0.02,11L)),3*n)
  header(">THREE Homo sapiens frequency")
  seed <- generate("acgt",c(0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008),5*n)
  close(output)
}
run(as.integer(commandArgs(TRUE)[1L]))
