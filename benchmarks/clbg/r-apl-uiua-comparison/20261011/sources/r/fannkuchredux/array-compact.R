# Serial bounded matrices; all permutations and prefix flips remain visible.
run <- function(n) {
  factorials <- c(1,cumprod(seq_len(n)))
  checksum <- 0
  maximum <- 0L
  for (start in seq.int(0L,factorials[n+1L]-1L,by=8192L)) {
    ranks <- seq.int(start,min(start+8191L,factorials[n+1L]-1L))
    remainder <- ranks
    permutation <- matrix(rep(seq_len(n),length(ranks)),n)
    if (n >= 2L) for (i in seq.int(n,2L,by=-1L)) {
      digit <- remainder %/% factorials[i]
      remainder <- remainder %% factorials[i]
      gather <- (rep(seq_len(i)-1L,length(ranks))+rep(digit,each=i)) %% i+1L+rep(seq_along(ranks)-1L,each=i)*n
      permutation[seq_len(i),] <- permutation[gather]
    }
    flips <- integer(length(ranks))
    active <- which(permutation[1L,] != 1L)
    while (length(active)) {
      first <- rep(permutation[1L,active],each=n)
      rows <- rep(seq_len(n),length(active))
      gather <- ifelse(rows <= first,first+1L-rows,rows)+rep(active-1L,each=n)*n
      permutation[,active] <- permutation[gather]
      flips[active] <- flips[active]+1L
      active <- active[permutation[1L,active] != 1L]
    }
    checksum <- checksum+sum(flips*ifelse(ranks %% 2L,-1L,1L))
    maximum <- max(maximum,flips)
  }
  cat(sprintf("%.0f\nPfannkuchen(%d) = %.0f\n",checksum,n,maximum))
}
run(as.integer(commandArgs(TRUE)[1L]))
