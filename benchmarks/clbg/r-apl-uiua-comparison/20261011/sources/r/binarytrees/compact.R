# Each node, including every leaf, is a fresh R list.
make <- function(depth) if (depth) list(make(depth-1L),make(depth-1L)) else list(NULL,NULL)
check <- function(tree) if (is.null(tree[[1L]])) 1 else 1+check(tree[[1L]])+check(tree[[2L]])
run <- function(n) {
  depth <- max(6L,n)
  cat(sprintf("stretch tree of depth %d\t check: %.0f\n",depth+1L,check(make(depth+1L))))
  gc(FALSE)
  long <- make(depth)
  for (d in seq.int(4L,depth,by=2L)) {
    iterations <- 2^(depth-d+4L)
    total <- sum(vapply(seq_len(iterations),function(i) check(make(d)),0.0))
    cat(sprintf("%.0f\t trees of depth %d\t check: %.0f\n",iterations,d,total))
  }
  cat(sprintf("long lived tree of depth %d\t check: %.0f\n",depth,check(long)))
}
run(as.integer(commandArgs(TRUE)[1L]))
