# Reverse complete records, then complement every IUB symbol and wrap at 60.
lines <- readLines(file("stdin"),warn=FALSE)
lines <- lines[!startsWith(lines,";")]
headers <- which(startsWith(lines,">"))
ends <- c(headers[-1L]-1L,length(lines))
output <- file("/dev/stdout","wb",raw=TRUE)
for (i in seq_along(headers)) {
  writeLines(lines[headers[i]],output)
  text <- if (ends[i]>headers[i]) paste0(lines[seq.int(headers[i]+1L,ends[i])],collapse="") else ""
  text <- paste0(rev(strsplit(chartr("ACGTUMRWSYKVHDBNacgtumrwsykvhdbn","TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN",text),"",fixed=TRUE)[[1L]]),collapse="")
  if (nchar(text)) writeLines(substring(text,seq.int(1L,nchar(text),by=60L),pmin(seq.int(1L,nchar(text),by=60L)+59L,nchar(text))),output)
}
close(output)
