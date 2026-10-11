# ------------------------------------------------------------------
# The Computer Language Shootout
# http://shootout.alioth.debian.org/
#
# Contributed by Leo Osvald
# ------------------------------------------------------------------

codes <- c(
    "A", "C", "G", "T", "U", "M", "R", "W", "S", "Y", "K", "V", "H", "D", "B",
    "N")
complements <- c(
    "T", "G", "C", "A", "A", "K", "Y", "W", "S", "R", "M", "B", "D", "H", "V",
    "N")
comp_map <- NULL
comp_map[codes] <- complements
comp_map[tolower(codes)] <- complements

reversecomplement_naive <- function(args) {
    f <- file(args[[1]], "r")
    while (length(s <- readLines(f, n=1, warn=FALSE))) {
        codes <- strsplit(s, split="")[[1]]
        if (codes[[1]] == '>')
            cat(s, "\n", sep="")
        else {
	    for (j in 1:length(codes))
	        codes[[j]] <- comp_map[[codes[[j]]]]
            cat(paste(codes, collapse=""), "\n", sep="")
        }
    }
    close(f)
}

execute <- function(n) {
    file <- paste("fasta/fasta", n, ".txt", sep="")
    reversecomplement_naive(file)
}
