# Fresh PSOCK workers avoid inheriting the retained tree; every node is allocated.
make <- compiler::cmpfun(function(depth) if (depth) list(make(depth-1L),make(depth-1L)) else list(NULL,NULL))
check <- compiler::cmpfun(function(tree) if (is.null(tree[[1L]])) 1 else 1+check(tree[[1L]])+check(tree[[2L]]))
task <- function(d,depth) {
  iterations <- 2^(depth-d+4L)
  total <- 0
  for (i in seq_len(iterations)) total <- total+check(make(d))
  sprintf("%.0f\t trees of depth %d\t check: %.0f\n",iterations,d,total)
}
run <- function(n) {
  depth <- max(6L,n)
  cat(sprintf("stretch tree of depth %d\t check: %.0f\n",depth+1L,check(make(depth+1L))))
  gc(FALSE)
  long <- make(depth)
  cluster <- parallel::makePSOCKcluster(8L)
  on.exit(parallel::stopCluster(cluster))
  parallel::clusterExport(cluster,c("make","check"),envir=globalenv())
  cat(unlist(parallel::parLapply(cluster,seq.int(4L,depth,by=2L),task,depth)),sep="")
  cat(sprintf("long lived tree of depth %d\t check: %.0f\n",depth,check(long)))
}
run(as.integer(commandArgs(TRUE)[1L]))
