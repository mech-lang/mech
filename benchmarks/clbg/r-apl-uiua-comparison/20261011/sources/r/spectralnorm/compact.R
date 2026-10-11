# Row-vectorized implicit canonical matrix; no cached matrix elements.
run <- function(n) {
  index <- seq_len(n)-1
  multiply <- function(u,transpose=FALSE) vapply(index,function(i) sum(u/((i+index)*(i+index+1)/2 + if (transpose) index+1 else i+1)),0.0)
  step <- function(u) multiply(multiply(u),TRUE)
  u <- rep(1,n)
  for (iteration in seq_len(10L)) {
    v <- step(u)
    u <- step(v)
  }
  cat(sprintf("%.9f\n",sqrt(sum(u*v)/sum(v*v))))
}
run(as.integer(commandArgs(TRUE)[1L]))
