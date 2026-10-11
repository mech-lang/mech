# Generic active-lane arrays, bounded row blocks, and eight standard fork workers.
run <- function(n) {
  output <- file("/dev/stdout","wb",raw=TRUE)
  writeBin(charToRaw(sprintf("P4\n%d %d\n",n,n)),output)
  row <- function(y) {
    cr <- 2*(seq_len(n)-1L)/n-1.5
    ci <- 2*y/n-1
    zr <- numeric(n)
    zi <- numeric(n)
    active <- seq_len(n)
    for (iteration in seq_len(50L)) {
      if (!length(active)) break
      real <- zr*zr-zi*zi+cr[active]
      zi <- 2*zr*zi+ci
      zr <- real
      keep <- zr*zr+zi*zi <= 4
      active <- active[keep]
      zr <- zr[keep]
      zi <- zi[keep]
    }
    bits <- numeric(n+((8L-n %% 8L) %% 8L))
    bits[active] <- rep(2^(7L:0L),length.out=n)[active]
    as.raw(colSums(matrix(bits,8L)))
  }
  for (start in seq.int(0L,n-1L,by=128L)) writeBin(unlist(parallel::mclapply(start:min(n-1L,start+127L),row,mc.cores=8L,mc.preschedule=TRUE),use.names=FALSE),output)
  close(output)
}
run(as.integer(commandArgs(TRUE)[1L]))
