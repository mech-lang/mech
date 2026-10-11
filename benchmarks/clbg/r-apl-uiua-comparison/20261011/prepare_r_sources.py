#!/usr/bin/env python3
"""Create transparent R adaptations; this is not a benchmark executor."""
from pathlib import Path
import json,hashlib,urllib.request
G=Path(__file__).resolve().parent
S=G/'sources'/'r'
def write(bench,variant,text):
 p=S/bench/(variant+'.R');p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text.strip()+'\n')
alu='GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA'
positions='c(0,0,0,4.84143144246472090,-1.16032004402742839,-0.103622044471123109,8.34336671824457987,4.12479856412430479,-0.403523417114321381,12.8943695621391310,-15.1111514016986312,-0.223307578892655734,15.3796971148509165,-25.9193146099879641,0.179258772950371181)'
velocities='c(0,0,0,1.66007664274403694e-3,7.69901118419740425e-3,-6.90460016972063023e-5,-2.76742510726862411e-3,4.99852801234917238e-3,2.30417297573763929e-5,2.96460137564761618e-3,2.37847173959480950e-3,-2.96589568540237556e-5,2.68067772490389322e-3,1.62824170038242295e-3,-9.51592254519715870e-5)'
masses='c(1,9.54791938424326609e-4,2.85885980666130812e-4,4.36624404335156298e-5,5.15138902046611451e-5) * (4*pi*pi)'
write('nbody','compact',f'''
# Canonical ten-pair application in base R; inspired by Leo Osvald's R shootout.
run <- function(n) {{
  p <- matrix({positions},5,3,byrow=TRUE)
  v <- matrix({velocities},5,3,byrow=TRUE) * 365.24
  m <- {masses}
  pair <- combn(5L,2L)
  incidence <- outer(seq_len(5L),pair[1L,],`==`) - outer(seq_len(5L),pair[2L,],`==`)
  v[1L,] <- -colSums(v*m)/m[1L]
  energy <- function() sum(m*rowSums(v*v)/2) - sum(m[pair[1L,]]*m[pair[2L,]]/sqrt(rowSums((p[pair[1L,],]-p[pair[2L,],])^2)))
  cat(sprintf("%.9f\\n",energy()))
  for (step in seq_len(n)) {{
    delta <- p[pair[1L,],] - p[pair[2L,],]
    distance <- sqrt(rowSums(delta*delta))
    force <- delta*(0.01/(distance*distance*distance))*m[pair[1L,]]*m[pair[2L,]]
    v <- v - (incidence %*% force)/m
    p <- p + 0.01*v
  }}
  cat(sprintf("%.9f\\n",energy()))
}}
run(as.integer(commandArgs(TRUE)[1L]))
''')
write('nbody','optimized',f'''
# All-pair vectorization adapted from the published nbody_3.R application.
run <- function(n) {{
  p <- matrix({positions},5,3,byrow=TRUE)
  v <- matrix({velocities},5,3,byrow=TRUE) * 365.24
  m <- {masses}
  i <- rep(seq_len(5L),5L)
  j <- rep(seq_len(5L),each=5L)
  pairs <- i < j
  diagonal <- i == j
  v[1L,] <- -colSums(v*m)/m[1L]
  energy <- function() sum(m*rowSums(v*v)/2) - sum((m[i]*m[j]/sqrt(rowSums((p[i,]-p[j,])^2)))[pairs])
  cat(sprintf("%.9f\\n",energy()))
  for (step in seq_len(n)) {{
    delta <- p[i,] - p[j,]
    d2 <- rowSums(delta*delta)
    d2[diagonal] <- Inf
    mag <- 0.01/(d2*sqrt(d2))
    v[,1L] <- v[,1L] - as.vector(matrix(delta[,1L]*mag,5L) %*% m)
    v[,2L] <- v[,2L] - as.vector(matrix(delta[,2L]*mag,5L) %*% m)
    v[,3L] <- v[,3L] - as.vector(matrix(delta[,3L]*mag,5L) %*% m)
    p <- p + 0.01*v
  }}
  cat(sprintf("%.9f\\n",energy()))
}}
run(as.integer(commandArgs(TRUE)[1L]))
''')
write('spectralnorm','compact',r'''
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
''')
write('spectralnorm','optimized',r'''
# Bounded blocks expose the same divisions to R's generic vector arithmetic.
run <- function(n) {
  index <- seq_len(n)-1
  multiply <- function(u,transpose=FALSE) {
    result <- numeric(n)
    for (start in seq.int(1L,n,by=128L)) {
      rows <- start:min(n,start+127L)
      s <- outer(rows-1,index,`+`)
      d <- s*(s+1)/2 + if (transpose) rep(index+1,each=length(rows)) else rows
      result[rows] <- rowSums(rep(u,each=length(rows))/d)
    }
    result
  }
  step <- function(u) multiply(multiply(u),TRUE)
  u <- rep(1,n)
  for (iteration in seq_len(10L)) {
    v <- step(u)
    u <- step(v)
  }
  cat(sprintf("%.9f\n",sqrt(sum(u*v)/sum(v*v))))
}
run(as.integer(commandArgs(TRUE)[1L]))
''')
write('binarytrees','compact',r'''
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
''')
write('binarytrees','optimized',r'''
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
''')
write('pidigits','compact',r'''
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
    digit <- (num*3L+acc) %/% den
    following <- (num*4L+acc) %/% den
    if (num > acc) next
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
''')
write('pidigits','optimized',r'''
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
''')
patterns='c("agggtaaa|tttaccct","[cgt]gggtaaa|tttaccc[acg]","a[act]ggtaaa|tttacc[agt]t","ag[act]gtaaa|tttac[agt]ct","agg[act]taaa|ttta[agt]cct","aggg[acg]aaa|ttt[cgt]ccct","agggt[cgt]aa|tt[acg]accct","agggta[cgt]a|t[acg]taccct","agggtaa[cgt]|[acg]ttaccct")'
regex=r'''
# The complete redux operations use ordinary PCRE regular expressions in base R.
text <- readChar(file("stdin","rb"),1000000000L,useBytes=TRUE)
initial <- nchar(text,type="bytes")
text <- gsub(">[^\n]*\n|\n","",text,perl=TRUE,useBytes=TRUE)
cleaned <- nchar(text,type="bytes")
for (pattern in PATTERNS) cat(pattern,sum(gregexpr(pattern,text,perl=TRUE,useBytes=TRUE)[[1L]] > 0L),"\n",sep=" ")
patterns <- c("tHa[Nt]","aND|caN|Ha[DS]|WaS","a[NSt]|BY","<[^>]*>","\\|[^|][^|]*\\|")
replacements <- c("<4>","<3>","<2>","|","-")
for (i in seq_along(patterns)) text <- gsub(patterns[i],replacements[i],text,perl=TRUE,useBytes=TRUE)
cat(sprintf("\n%d\n%d\n%d\n",initial,cleaned,nchar(text,type="bytes")))
'''.replace('PATTERNS',patterns)
# Correct line separator without an extra space before the newline.
regex=regex.replace('cat(pattern,sum(gregexpr(pattern,text,perl=TRUE,useBytes=TRUE)[[1L]] > 0L),"\\n",sep=" ")','cat(sprintf("%s %d\\n",pattern,sum(gregexpr(pattern,text,perl=TRUE,useBytes=TRUE)[[1L]] > 0L)))')
write('regexredux','compact',regex)
write('regexredux','optimized',regex.replace('patterns <-', '# Base PCRE is already the same native generic regex engine.\npatterns <-',1))
write('fasta','compact',f'''
# Canonical binary64 division and CDF, with every RNG step executed.
seed <- 42L
alu <- "{alu}"
output <- file("/dev/stdout","wb",raw=TRUE)
line <- function(x) writeBin(charToRaw(paste0(x,"\\n")),output)
generate <- function(letters,probabilities,n) {{
  cdf <- cumsum(probabilities)
  letters <- strsplit(letters,"",fixed=TRUE)[[1L]]
  for (start in seq.int(0,n-1,by=60L)) {{
    random <- numeric(min(60L,n-start))
    for (i in seq_along(random)) {{
      seed <<- (seed*3877L+29573L) %% 139968L
      random[i] <- seed/139968
    }}
    line(paste0(letters[findInterval(random,cdf)+1L],collapse=""))
  }}
}}
n <- as.integer(commandArgs(TRUE)[1L])
line(">ONE Homo sapiens alu")
for (start in seq.int(0,2*n-1,by=60L)) line(paste0(substring(alu,(start+seq_len(min(60L,2*n-start))-1) %% nchar(alu)+1L,(start+seq_len(min(60L,2*n-start))-1) %% nchar(alu)+1L),collapse=""))
line(">TWO IUB ambiguity codes")
generate("acgtBDHKMNRSVWY",c(0.27,0.12,0.12,0.27,rep(0.02,11L)),3*n)
line(">THREE Homo sapiens frequency")
generate("acgt",c(0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008),5*n)
close(output)
''')
write('fasta','optimized',f'''
# Generic affine-prefix scans materialize every RNG seed; no period/output cache.
run <- function(n) {{
  seed <- 42L
  output <- file("/dev/stdout","wb",raw=TRUE)
  emit <- function(x) writeBin(x,output)
  header <- function(x) emit(charToRaw(paste0(x,"\\n")))
  block <- 60L*4096L
  wrap <- function(x) {{
    length <- length(x)
    bytes <- rep(as.raw(10L),length+ceiling(length/60L))
    bytes[seq_len(length)+(seq_len(length)-1L) %/% 60L] <- x
    emit(bytes)
  }}
  generate <- function(letters,probabilities,n) {{
    cdf <- cumsum(probabilities)
    letters <- charToRaw(letters)
    for (start in seq.int(0,n-1,by=block)) {{
      size <- min(block,n-start)
      multipliers <- rep(3877.0,size)
      increments <- rep(29573.0,size)
      offset <- 1L
      while (offset < size) {{
        suffix <- seq.int(offset+1L,size)
        prefix <- suffix-offset
        increments[suffix] <- (multipliers[suffix]*increments[prefix]+increments[suffix]) %% 139968
        multipliers[suffix] <- (multipliers[suffix]*multipliers[prefix]) %% 139968
        offset <- offset*2L
      }}
      seeds <- (multipliers*seed+increments) %% 139968
      random <- seeds/139968
      seed <- seeds[size]
      wrap(letters[findInterval(random,cdf)+1L])
    }}
    seed
  }}
  alu <- charToRaw("{alu}")
  header(">ONE Homo sapiens alu")
  for (start in seq.int(0,2*n-1,by=block)) wrap(alu[(start+seq_len(min(block,2*n-start))-1) %% length(alu)+1L])
  header(">TWO IUB ambiguity codes")
  seed <- generate("acgtBDHKMNRSVWY",c(0.27,0.12,0.12,0.27,rep(0.02,11L)),3*n)
  header(">THREE Homo sapiens frequency")
  seed <- generate("acgt",c(0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008),5*n)
  close(output)
}}
run(as.integer(commandArgs(TRUE)[1L]))
''')
write('knucleotide','compact',r'''
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
''')
write('knucleotide','optimized',r'''
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
''')
write('revcomp','compact',r'''
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
''')
write('revcomp','optimized',r'''
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
''')
write('mandelbrot','compact',r'''
# Real binary64 recurrence and the canonical P4 bitmap in base R.
run <- function(n) {
  output <- file("/dev/stdout","wb",raw=TRUE)
  writeBin(charToRaw(sprintf("P4\n%d %d\n",n,n)),output)
  cr <- 2*(seq_len(n)-1L)/n-1.5
  weights <- rep(2^(7L:0L),length.out=n)
  for (y in seq_len(n)-1L) {
    zr <- numeric(n)
    zi <- numeric(n)
    inside <- rep(TRUE,n)
    for (iteration in seq_len(50L)) {
      real <- zr*zr-zi*zi+cr
      zi <- 2*zr*zi+2*y/n-1
      zr <- real
      inside <- inside & (zr*zr+zi*zi <= 4)
    }
    writeBin(as.raw(colSums(matrix(c(as.integer(inside)*weights,rep(0,(8L-n %% 8L) %% 8L)),8L))),output)
  }
  close(output)
}
run(as.integer(commandArgs(TRUE)[1L]))
''')
write('mandelbrot','optimized',r'''
# Generic active-lane arrays, bounded row blocks, and eight standard fork workers.
run <- function(n) {
  output <- file("/dev/stdout","wb",raw=TRUE)
  writeBin(charToRaw(sprintf("P4\n%d %d\n",n,n)),output)
  row <- function(y) {
    cr <- 2*(seq_len(n)-1L)/n-1.5
    ci <- 2*y/n-1
    zr <- numeric(n)
    zi <- numeric(n)
    active <- seq_len(n)
    for (iteration in seq_len(50L)) {
      if (!length(active)) break
      real <- zr*zr-zi*zi+cr[active]
      zi <- 2*zr*zi+ci
      zr <- real
      keep <- zr*zr+zi*zi <= 4
      active <- active[keep]
      zr <- zr[keep]
      zi <- zi[keep]
    }
    bits <- numeric(n+((8L-n %% 8L) %% 8L))
    bits[active] <- rep(2^(7L:0L),length.out=n)[active]
    as.raw(colSums(matrix(bits,8L)))
  }
  for (start in seq.int(0L,n-1L,by=128L)) writeBin(unlist(parallel::mclapply(start:min(n-1L,start+127L),row,mc.cores=8L,mc.preschedule=TRUE),use.names=FALSE),output)
  close(output)
}
run(as.integer(commandArgs(TRUE)[1L]))
''')
write('fannkuchredux','compact',r'''
# Canonical permutation order, full prefix flips and alternating checksum.
run <- function(n) {
  permutation <- seq_len(n)
  count <- integer(n)
  r <- n
  rank <- 0L
  checksum <- 0
  maximum <- 0L
  repeat {
    while (r != 1L) {
      count[r] <- r
      r <- r-1L
    }
    work <- permutation
    flips <- 0L
    while (work[1L] != 1L) {
      k <- work[1L]
      work[seq_len(k)] <- rev(work[seq_len(k)])
      flips <- flips+1L
    }
    checksum <- checksum + if (rank %% 2L) -flips else flips
    maximum <- max(maximum,flips)
    repeat {
      if (r == n) {
        cat(sprintf("%.0f\nPfannkuchen(%d) = %d\n",checksum,n,maximum))
        return(invisible(NULL))
      }
      permutation[seq_len(r+1L)] <- permutation[c(seq.int(2L,r+1L),1L)]
      count[r+1L] <- count[r+1L]-1L
      if (count[r+1L]>0L) break
      r <- r+1L
    }
    rank <- rank+1L
  }
}
run(as.integer(commandArgs(TRUE)[1L]))
''')
write('fannkuchredux','optimized',r'''
# Independent factoradic ranges use standard fork workers; every permutation flips.
run <- function(n) {
  factorials <- cumprod(c(1,seq_len(n)))
  total <- factorials[n+1L]
  if (n < 2L) {
    cat(sprintf("0\nPfannkuchen(%d) = 0\n",n))
    return(invisible(NULL))
  }
  workers <- min(8L,total)
  ranges <- floor(seq.int(0L,workers)*total/workers)
  prefix <- lapply(seq_len(n),seq_len)
  reversal <- lapply(prefix,rev)
  rotation <- lapply(prefix,function(x) c(x[-1L],1L))
  task <- function(worker) {
    first <- ranges[worker]
    last <- ranges[worker+1L]
    permutation <- seq_len(n)
    count <- integer(n)
    rank <- first
    for (i in seq.int(n-1L,1L,by=-1L)) {
      digit <- rank %/% factorials[i+1L]
      count[i+1L] <- digit
      rank <- rank %% factorials[i+1L]
      if (digit) permutation[seq_len(i+1L)] <- permutation[c(seq.int(digit+1L,i+1L),seq_len(digit))]
    }
    checksum <- 0
    maximum <- 0L
    for (rank in seq.int(first,last-1L)) {
      work <- permutation
      flips <- 0L
      while (work[1L] != 1L) {
        k <- work[1L]
        work[prefix[[k]]] <- work[reversal[[k]]]
        flips <- flips+1L
      }
      checksum <- checksum + if (rank %% 2L) -flips else flips
      if (flips > maximum) maximum <- flips
      if (rank+1L == last) break
      for (i in seq.int(2L,n)) {
        permutation[prefix[[i]]] <- permutation[rotation[[i]]]
        count[i] <- count[i]+1L
        if (count[i] < i) break
        count[i] <- 0L
      }
    }
    c(checksum,maximum)
  }
  result <- parallel::mclapply(seq_len(workers),task,mc.cores=workers,mc.preschedule=TRUE)
  cat(sprintf("%.0f\nPfannkuchen(%d) = %.0f\n",sum(vapply(result,`[`,0.0,1L)),n,max(vapply(result,`[`,0.0,2L))))
}
run(as.integer(commandArgs(TRUE)[1L]))
''')

write("regexredux","optimized",(S/"regexredux/compact.R").read_text())

write('fannkuchredux','array-batched','''
# Bounded permutation matrices and indexed prefix reversals; no native kernel.
run <- function(n) {
  factorials <- c(1,cumprod(seq_len(n)))
  total <- factorials[n+1L]
  workers <- min(8L,total)
  ranges <- floor(seq.int(0L,workers)*total/workers)
  task <- function(worker) {
    checksum <- 0
    maximum <- 0L
    for (start in seq.int(ranges[worker],ranges[worker+1L]-1L,by=8192L)) {
      ranks <- seq.int(start,min(start+8191L,ranges[worker+1L]-1L))
      remainder <- ranks
      permutation <- matrix(rep(seq_len(n),length(ranks)),n)
      if (n >= 2L) for (i in seq.int(n,2L,by=-1L)) {
        digit <- remainder %/% factorials[i]
        remainder <- remainder %% factorials[i]
        gather <- (rep(seq_len(i)-1L,length(ranks))+rep(digit,each=i)) %% i+1L+rep(seq_along(ranks)-1L,each=i)*n
        permutation[seq_len(i),] <- permutation[gather]
      }
      flips <- integer(length(ranks))
      active <- which(permutation[1L,] != 1L)
      while (length(active)) {
        first <- rep(permutation[1L,active],each=n)
        rows <- rep(seq_len(n),length(active))
        gather <- ifelse(rows <= first,first+1L-rows,rows)+rep(active-1L,each=n)*n
        permutation[,active] <- permutation[gather]
        flips[active] <- flips[active]+1L
        active <- active[permutation[1L,active] != 1L]
      }
      checksum <- checksum+sum(flips*ifelse(ranks %% 2L,-1L,1L))
      maximum <- max(maximum,flips)
    }
    c(checksum,maximum)
  }
  result <- parallel::mclapply(seq_len(workers),task,mc.cores=workers,mc.preschedule=TRUE)
  cat(sprintf("%.0f\\nPfannkuchen(%d) = %.0f\\n",sum(vapply(result,`[`,0.0,1L)),n,max(vapply(result,`[`,0.0,2L))))
}
run(as.integer(commandArgs(TRUE)[1L]))
''')

write('fannkuchredux','array-compact','''
# Serial bounded matrices; all permutations and prefix flips remain visible.
run <- function(n) {
  factorials <- c(1,cumprod(seq_len(n)))
  checksum <- 0
  maximum <- 0L
  for (start in seq.int(0L,factorials[n+1L]-1L,by=8192L)) {
    ranks <- seq.int(start,min(start+8191L,factorials[n+1L]-1L))
    remainder <- ranks
    permutation <- matrix(rep(seq_len(n),length(ranks)),n)
    if (n >= 2L) for (i in seq.int(n,2L,by=-1L)) {
      digit <- remainder %/% factorials[i]
      remainder <- remainder %% factorials[i]
      gather <- (rep(seq_len(i)-1L,length(ranks))+rep(digit,each=i)) %% i+1L+rep(seq_along(ranks)-1L,each=i)*n
      permutation[seq_len(i),] <- permutation[gather]
    }
    flips <- integer(length(ranks))
    active <- which(permutation[1L,] != 1L)
    while (length(active)) {
      first <- rep(permutation[1L,active],each=n)
      rows <- rep(seq_len(n),length(active))
      gather <- ifelse(rows <= first,first+1L-rows,rows)+rep(active-1L,each=n)*n
      permutation[,active] <- permutation[gather]
      flips[active] <- flips[active]+1L
      active <- active[permutation[1L,active] != 1L]
    }
    checksum <- checksum+sum(flips*ifelse(ranks %% 2L,-1L,1L))
    maximum <- max(maximum,flips)
  }
  cat(sprintf("%.0f\\nPfannkuchen(%d) = %.0f\\n",checksum,n,maximum))
}
run(as.integer(commandArgs(TRUE)[1L]))
''')
