# Both unbounded integer extracts are performed for every candidate attempt.
suppressPackageStartupMessages(library(gmp))
run <- function(n) {
  num <- as.bigz(1)
  acc <- as.bigz(0)
  den <- as.bigz(1)
  k <- 0L
  printed <- 0L
  while (printed < n) {
    k <- k+1L
    acc <- (acc+num*2L)*(2L*k+1L)
    den <- den*(2L*k+1L)
    num <- num*k
    if (num > acc) next
    digit <- (num*3L+acc) %/% den
    following <- (num*4L+acc) %/% den
    if (digit != following) next
    cat(as.character(digit))
    printed <- printed+1L
    if (printed %% 10L == 0L) cat(sprintf("\t:%d\n",printed))
    acc <- (acc-den*digit)*10L
    num <- num*10L
  }
  if (n %% 10L) cat(strrep(" ",10L-n %% 10L),sprintf("\t:%d\n",n),sep="")
}
run(as.integer(commandArgs(TRUE)[1L]))
