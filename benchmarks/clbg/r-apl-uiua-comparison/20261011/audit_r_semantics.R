# Executable fixtures for complete maps and actual tree node identity; not timed.
root <- commandArgs(TRUE)[1L]
dna <- "AAAAAACCCCGGGGTTTTTCCCAAAGTTGCGGGTATTTTAATTTATAGT"
for (variant in c("compact","optimized")) {
  code <- parse(file.path(root,"sources","r","knucleotide",paste0(variant,".R")))
  assignments <- if (variant=="compact") as.list(code) else as.list(code[[1L]][[3L]][[3L]])
  frequency <- Filter(function(x) is.call(x) && identical(x[[1L]],as.name("<-")) && identical(x[[2L]],as.name("frequency")),assignments)[[1L]]
  local <- new.env()
  local$dna <- dna
  eval(frequency,local)
  for (width in c(1L,2L,3L,4L,6L,12L,18L)) {
    counts <- local$frequency(width)
    for (key in names(counts)) cat(sprintf("map\t%s\t%d\t%s\t%.0f\n",variant,width,key,counts[key]))
  }
  tree_code <- parse(file.path(root,"sources","r","binarytrees",paste0(variant,".R")))
  tree_env <- new.env()
  for (expression in tree_code) if (is.call(expression) && identical(expression[[1L]],as.name("<-")) && as.character(expression[[2L]]) %in% c("make","check")) eval(expression,tree_env)
  tree <- tree_env$make(5L)
  addresses <- character()
  walk <- function(node) {
    addresses <<- c(addresses,tracemem(node))
    if (!is.null(node[[1L]])) {
      walk(node[[1L]])
      walk(node[[2L]])
    }
  }
  walk(tree)
  stopifnot(length(addresses)==63L,length(unique(addresses))==63L,tree_env$check(tree)==63)
  cat(sprintf("tree\t%s\t63\t63\n",variant))
}
stopifnot(!capabilities("long.double"))
for (probabilities in list(c(0.27,0.12,0.12,0.27,rep(0.02,11L)),c(0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008))) {
  sum <- 0.0
  sequential <- vapply(probabilities,function(p) { sum <<- sum+p; sum },0.0)
  stopifnot(identical(cumsum(probabilities),sequential))
}
cat("cdf\tbinary64\tunscaled\tsequential-rounding\n")
