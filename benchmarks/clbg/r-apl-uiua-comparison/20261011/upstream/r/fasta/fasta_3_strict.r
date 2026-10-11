# ------------------------------------------------------------------
# The Computer Language Shootout
# http://shootout.alioth.debian.org/
#
# Contributed by Leo Osvald
# ------------------------------------------------------------------
width <- 60L
myrandom_last <- 42L
myrandom <- function(m) {
    myrandom_last <<- (myrandom_last * 3877L + 29573L) %% 139968L
    return(m * myrandom_last / 139968)
}
myrandom <- rir.annotateDepromised(myrandom)


alu <- paste(
    "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG",
    "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA",
    "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT",
    "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA",
    "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG",
    "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC",
    "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA",
    sep="", collapse="")

iub <- matrix(c(
    c(0.27, 'a'),
    c(0.12, 'c'),
    c(0.12, 'g'),
    c(0.27, 't'),
    c(0.02, 'B'),
    c(0.02, 'D'),
    c(0.02, 'H'),
    c(0.02, 'K'),
    c(0.02, 'M'),
    c(0.02, 'N'),
    c(0.02, 'R'),
    c(0.02, 'S'),
    c(0.02, 'V'),
    c(0.02, 'W'),
    c(0.02, 'Y')
), 2)

homosapiens <- matrix(c(
    c(0.3029549426680, 'a'),
    c(0.1979883004921, 'c'),
    c(0.1975473066391, 'g'),
    c(0.3015094502008, 't')
), 2)

repeat_fasta <- function(s, count) {
    chars <- strsplit(s, split="")[[1]]
    len <- nchar(s)
    s2 <- c(chars, chars[1:width])
    pos <- 1L
    while (count) {
        line <- min(width, count)
        next_pos <- pos + line
        cat(s2[pos:(next_pos - 1)], "\n", sep="")
        pos <- next_pos
        if (pos > len) pos <- pos - len
        count <- count - line
    }
}
repeat_fasta <- rir.annotateDepromised(repeat_fasta)

random_fasta <- function(genelist, count) {
    psum = cumsum(genelist[1,])
    n = length(psum)
    while (count) {
	line = min(width, count) 
        rs <- double(line)
        for (i in 1:line)
            rs[[i]] <- myrandom(1)

	# Binary search (vectorized)
	inds <- 1:line
       	lo <- rep.int(1L, line); hi <- rep.int(n, line)
	mid <- integer(line)
	repeat {
	    mid[inds] <- (lo[inds] + hi[inds] - 1L) %/% 2
	    ge_inds = which(psum[mid] >= rs)
	    hi[ge_inds] <- mid[ge_inds]
	    lt_inds = which(psum[mid] < rs)
	    lo[lt_inds] <- mid[lt_inds] + 1L
	    inds <- which(lo < hi)
	    if (!length(inds))
	        break
	}

	cat(genelist[2, hi], "\n", sep='')
	count <- count - line
    }

}
random_fasta <- rir.annotateDepromised(random_fasta)


fasta_3 <- function(args) {
    n = if (length(args)) as.integer(args[[1]]) else 1000L
    cat(">ONE Homo sapiens alu\n")
    repeat_fasta(alu, 2 * n)
    cat(">TWO IUB ambiguity codes\n")
    random_fasta(iub, 3L * n)
    cat(">THREE Homo sapiens frequency\n")
    random_fasta(homosapiens, 5L * n)
}
fasta_3 <- rir.annotateDepromised(fasta_3)

execute <- function(n) {
    fasta_3(n)
}
