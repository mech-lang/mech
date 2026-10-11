# Bounded generic raw-array blocks reverse and translate complete records.
run <- function() {
  lines <- readLines(file("stdin"),warn=FALSE)
  lines <- lines[!startsWith(lines,";")]
  headers <- which(startsWith(lines,">"))
  ends <- c(headers[-1L]-1L,length(lines))
  table <- as.raw(seq.int(0L,255L))
  table[as.integer(charToRaw("ACGTUMRWSYKVHDBNacgtumrwsykvhdbn"))+1L] <- charToRaw("TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN")
  output <- file("/dev/stdout","wb",raw=TRUE)
  block <- 60L*4096L
  for (i in seq_along(headers)) {
    writeBin(charToRaw(paste0(lines[headers[i]],"\n")),output)
    text <- if (ends[i]>headers[i]) charToRaw(paste0(lines[seq.int(headers[i]+1L,ends[i])],collapse="")) else raw()
    n <- length(text)
    if (n) for (start in seq.int(0L,n-1L,by=block)) {
      size <- min(block,n-start)
      translated <- table[as.integer(text[n-start-seq_len(size)+1L])+1L]
      bytes <- rep(as.raw(10L),size+ceiling(size/60L))
      bytes[seq_len(size)+(seq_len(size)-1L) %/% 60L] <- translated
      writeBin(bytes,output)
    }
  }
  close(output)
}
run()
