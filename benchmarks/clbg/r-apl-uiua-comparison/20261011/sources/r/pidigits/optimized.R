# Generic GMP vectors batch the two distinct quotient extracts, without shortcut.
suppressPackageStartupMessages(library(gmp))
run <- function(n) {
  num <- as.bigz(1)
  acc <- as.bigz(0)
  den <- as.bigz(1)
  k <- 0L
  printed <- 0L
  line <- character(10L)
  multipliers <- as.bigz(c(3L,4L))
  while (printed < n) {
    k <- k+1L
    odd <- 2L*k+1L
    acc <- (acc+num*2L)*odd
    den <- den*odd
    num <- num*k
    if (num > acc) next
    extracts <- (num*multipliers+acc) %/% den
    if (extracts[1L] != extracts[2L]) next
    line[printed %% 10L+1L] <- as.character(extracts[1L])
    printed <- printed+1L
    if (printed %% 10L == 0L) cat(paste0(line,collapse=""),sprintf("\t:%d\n",printed),sep="")
    acc <- (acc-den*extracts[1L])*10L
    num <- num*10L
  }
  if (n %% 10L) cat(paste0(line[seq_len(n %% 10L)],collapse=""),strrep(" ",10L-n %% 10L),sprintf("\t:%d\n",n),sep="")
}
run(as.integer(commandArgs(TRUE)[1L]))
