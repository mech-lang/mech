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

reversecomplement_2 <- function(args) {
    f <- file(args[[1]], "r")
    lines <- readLines(f)
    for (i in 1:length(lines)) {
        codes <- strsplit(lines[[i]], split="")[[1]]
        if (codes[[1]] == '>')
            cat(lines[[i]], "\n", sep="")
        else {
            cat(paste(comp_map[codes], collapse=""), "\n",
                sep="")
        }
    }
    close(f)
}

execute <- function(n) {
    file <- paste("fasta/fasta", n, ".txt", sep="")
    reversecomplement_2(file)
}
