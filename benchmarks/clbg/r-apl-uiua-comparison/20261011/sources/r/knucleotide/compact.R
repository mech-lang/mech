# Complete growing maps for all seven widths, built in bounded input chunks.
lines <- readLines(file("stdin"),warn=FALSE)
lines <- lines[!startsWith(lines,";")]
start <- match(TRUE,startsWith(lines,">THREE"),nomatch=length(lines))
end <- which(startsWith(lines,">") & seq_along(lines)>start)
last <- if (length(end)) end[1L]-1L else length(lines)
dna <- if (last>start) toupper(paste0(lines[seq.int(start+1L,last)],collapse="")) else ""
frequency <- function(width) {
  total <- numeric()
  windows <- nchar(dna,type="bytes")-width+1L
  if (windows < 1L) return(total)
  for (start in seq.int(1L,windows,by=250000L)) {
    positions <- start:min(windows,start+249999L)
    piece <- table(substring(dna,positions,positions+width-1L))
    keys <- union(names(total),names(piece))
    total <- setNames(replace(total[keys],is.na(total[keys]),0)+replace(as.numeric(piece[keys]),is.na(piece[keys]),0),keys)
  }
  total
}
queries <- c("GGT","GGTA","GGTATT","GGTATTTTAATT","GGTATTTTAATTTATAGT")
for (width in c(1L,2L,3L,4L,6L,12L,18L)) {
  counts <- frequency(width)
  if (width <= 2L) {
    keys <- names(counts)[order(-counts,as.character(names(counts)),method="radix")]
    cat(sprintf("%s %.3f\n",keys,100*counts[keys]/sum(counts)),sep="")
    cat("\n")
  } else cat(sprintf("%.0f\t%s\n",if (queries[match(width,c(3L,4L,6L,12L,18L))] %in% names(counts)) counts[queries[match(width,c(3L,4L,6L,12L,18L))]] else 0,queries[match(width,c(3L,4L,6L,12L,18L))]))
}
