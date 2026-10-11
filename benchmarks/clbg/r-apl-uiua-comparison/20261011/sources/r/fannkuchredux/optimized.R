# Independent factoradic ranges use standard fork workers; every permutation flips.
run <- function(n) {
  factorials <- cumprod(c(1,seq_len(n)))
  total <- factorials[n+1L]
  if (n < 2L) {
    cat(sprintf("0\nPfannkuchen(%d) = 0\n",n))
    return(invisible(NULL))
  }
  workers <- min(8L,total)
  ranges <- floor(seq.int(0L,workers)*total/workers)
  prefix <- lapply(seq_len(n),seq_len)
  reversal <- lapply(prefix,rev)
  rotation <- lapply(prefix,function(x) c(x[-1L],1L))
  task <- function(worker) {
    first <- ranges[worker]
    last <- ranges[worker+1L]
    permutation <- seq_len(n)
    count <- integer(n)
    rank <- first
    for (i in seq.int(n-1L,1L,by=-1L)) {
      digit <- rank %/% factorials[i+1L]
      count[i+1L] <- digit
      rank <- rank %% factorials[i+1L]
      if (digit) permutation[seq_len(i+1L)] <- permutation[c(seq.int(digit+1L,i+1L),seq_len(digit))]
    }
    checksum <- 0
    maximum <- 0L
    for (rank in seq.int(first,last-1L)) {
      work <- permutation
      flips <- 0L
      while (work[1L] != 1L) {
        k <- work[1L]
        work[prefix[[k]]] <- work[reversal[[k]]]
        flips <- flips+1L
      }
      checksum <- checksum + if (rank %% 2L) -flips else flips
      if (flips > maximum) maximum <- flips
      if (rank+1L == last) break
      for (i in seq.int(2L,n)) {
        permutation[prefix[[i]]] <- permutation[rotation[[i]]]
        count[i] <- count[i]+1L
        if (count[i] < i) break
        count[i] <- 0L
      }
    }
    c(checksum,maximum)
  }
  result <- parallel::mclapply(seq_len(workers),task,mc.cores=workers,mc.preschedule=TRUE)
  cat(sprintf("%.0f\nPfannkuchen(%d) = %.0f\n",sum(vapply(result,`[`,0.0,1L)),n,max(vapply(result,`[`,0.0,2L))))
}
run(as.integer(commandArgs(TRUE)[1L]))
