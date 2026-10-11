# Canonical binary64 division and CDF, with every RNG step executed.
seed <- 42L
alu <- "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
output <- file("/dev/stdout","wb",raw=TRUE)
line <- function(x) writeBin(charToRaw(paste0(x,"\n")),output)
generate <- function(letters,probabilities,n) {
  cdf <- cumsum(probabilities)
  letters <- strsplit(letters,"",fixed=TRUE)[[1L]]
  for (start in seq.int(0,n-1,by=60L)) {
    random <- numeric(min(60L,n-start))
    for (i in seq_along(random)) {
      seed <<- (seed*3877L+29573L) %% 139968L
      random[i] <- seed/139968
    }
    line(paste0(letters[findInterval(random,cdf)+1L],collapse=""))
  }
}
n <- as.integer(commandArgs(TRUE)[1L])
line(">ONE Homo sapiens alu")
for (start in seq.int(0,2*n-1,by=60L)) line(paste0(substring(alu,(start+seq_len(min(60L,2*n-start))-1) %% nchar(alu)+1L,(start+seq_len(min(60L,2*n-start))-1) %% nchar(alu)+1L),collapse=""))
line(">TWO IUB ambiguity codes")
generate("acgtBDHKMNRSVWY",c(0.27,0.12,0.12,0.27,rep(0.02,11L)),3*n)
line(">THREE Homo sapiens frequency")
generate("acgt",c(0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008),5*n)
close(output)
