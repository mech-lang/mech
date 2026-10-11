# Generic match/tabulate form: grow the dictionary for every observed window.
run <- function() {
  lines <- readLines(file("stdin"),warn=FALSE)
  lines <- lines[!startsWith(lines,";")]
  start <- match(TRUE,startsWith(lines,">THREE"),nomatch=length(lines))
  end <- which(startsWith(lines,">") & seq_along(lines)>start)
  last <- if (length(end)) end[1L]-1L else length(lines)
  dna <- if (last>start) toupper(paste0(lines[seq.int(start+1L,last)],collapse="")) else ""
  rm(lines)
  gc(FALSE)
  frequency <- function(width) {
    keys <- character()
    counts <- numeric()
    windows <- nchar(dna,type="bytes")-width+1L
    if (windows < 1L) return(counts)
    for (start in seq.int(1L,windows,by=1000000L)) {
      positions <- start:min(windows,start+999999L)
      words <- substring(dna,positions,positions+width-1L)
      known <- match(words,keys)
      fresh <- unique(words[is.na(known)])
      if (length(fresh)) {
        keys <- c(keys,fresh)
        counts <- c(counts,numeric(length(fresh)))
        known[is.na(known)] <- match(words[is.na(known)],keys)
      }
      counts <- counts+tabulate(known,nbins=length(keys))
    }
    setNames(counts,keys)
  }
  queries <- c("GGT","GGTA","GGTATT","GGTATTTTAATT","GGTATTTTAATTTATAGT")
  for (width in c(1L,2L,3L,4L,6L,12L,18L)) {
    counts <- frequency(width)
    if (width <= 2L) {
      keys <- names(counts)[order(-counts,as.character(names(counts)),method="radix")]
      cat(sprintf("%s %.3f\n",keys,100*counts[keys]/sum(counts)),sep="")
      cat("\n")
    } else {
      query <- queries[match(width,c(3L,4L,6L,12L,18L))]
      cat(sprintf("%.0f\t%s\n",if (query %in% names(counts)) counts[query] else 0,query))
    }
  }
}
run()
