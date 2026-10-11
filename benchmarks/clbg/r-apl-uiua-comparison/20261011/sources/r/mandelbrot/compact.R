# Real binary64 recurrence and the canonical P4 bitmap in base R.
run <- function(n) {
  output <- file("/dev/stdout","wb",raw=TRUE)
  writeBin(charToRaw(sprintf("P4\n%d %d\n",n,n)),output)
  cr <- 2*(seq_len(n)-1L)/n-1.5
  weights <- rep(2^(7L:0L),length.out=n)
  for (y in seq_len(n)-1L) {
    zr <- numeric(n)
    zi <- numeric(n)
    inside <- rep(TRUE,n)
    for (iteration in seq_len(50L)) {
      real <- zr*zr-zi*zi+cr
      zi <- 2*zr*zi+2*y/n-1
      zr <- real
      inside <- inside & (zr*zr+zi*zi <= 4)
    }
    writeBin(as.raw(colSums(matrix(c(as.integer(inside)*weights,rep(0,(8L-n %% 8L) %% 8L)),8L))),output)
  }
  close(output)
}
run(as.integer(commandArgs(TRUE)[1L]))
