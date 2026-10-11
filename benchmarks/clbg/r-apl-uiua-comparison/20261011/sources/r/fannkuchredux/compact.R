# Canonical permutation order, full prefix flips and alternating checksum.
run <- function(n) {
  permutation <- seq_len(n)
  count <- integer(n)
  r <- n
  rank <- 0L
  checksum <- 0
  maximum <- 0L
  repeat {
    while (r != 1L) {
      count[r] <- r
      r <- r-1L
    }
    work <- permutation
    flips <- 0L
    while (work[1L] != 1L) {
      k <- work[1L]
      work[seq_len(k)] <- rev(work[seq_len(k)])
      flips <- flips+1L
    }
    checksum <- checksum + if (rank %% 2L) -flips else flips
    maximum <- max(maximum,flips)
    repeat {
      if (r == n) {
        cat(sprintf("%.0f\nPfannkuchen(%d) = %d\n",checksum,n,maximum))
        return(invisible(NULL))
      }
      permutation[seq_len(r+1L)] <- permutation[c(seq.int(2L,r+1L),1L)]
      count[r+1L] <- count[r+1L]-1L
      if (count[r+1L]>0L) break
      r <- r+1L
    }
    rank <- rank+1L
  }
}
run(as.integer(commandArgs(TRUE)[1L]))
