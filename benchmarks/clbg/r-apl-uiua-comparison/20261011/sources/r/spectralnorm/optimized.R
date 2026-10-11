# Bounded blocks expose the same divisions to R's generic vector arithmetic.
run <- function(n) {
  index <- seq_len(n)-1
  multiply <- function(u,transpose=FALSE) {
    result <- numeric(n)
    for (start in seq.int(1L,n,by=128L)) {
      rows <- start:min(n,start+127L)
      s <- outer(rows-1,index,`+`)
      d <- s*(s+1)/2 + if (transpose) rep(index+1,each=length(rows)) else rows
      result[rows] <- rowSums(rep(u,each=length(rows))/d)
    }
    result
  }
  step <- function(u) multiply(multiply(u),TRUE)
  u <- rep(1,n)
  for (iteration in seq_len(10L)) {
    v <- step(u)
    u <- step(v)
  }
  cat(sprintf("%.9f\n",sqrt(sum(u*v)/sum(v*v))))
}
run(as.integer(commandArgs(TRUE)[1L]))
