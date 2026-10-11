# ------------------------------------------------------------------
# The Computer Language Shootout
# http://shootout.alioth.debian.org/
#
# Contributed by Leo Osvald
# ------------------------------------------------------------------

spectralnorm_alt_3 <- function(args) {
    n = if (length(args)) as.integer(args[[1]]) else 100L
    options(digits=10)

    eval_A <- function(i, j) eval_A_cache[[i, j]]
    eval_A <- rir.annotateDepromised(eval_A)

    eval_A_times_u <- function(u) {
        #    eval_A_mat <- outer(seq(n), seq(n), FUN=eval_A)
        eval_A_mat <- matrix(0, n, n)
        for (i in 1:n)
            for (j in 1:n)
                eval_A_mat[[i, j]] <- eval_A(i, j)
        return(u %*% t(eval_A_mat))
    }
    eval_A_times_u <- rir.annotateDepromised(eval_A_times_u)


    eval_At_times_u <- function(u) {
        # eval_At_mat <- t(outer(seq(n), seq(n), FUN=eval_A))
        eval_At_mat <- matrix(0, n, n)
        for (i in 1:n)
            for (j in 1:n)
                eval_At_mat[[i, j]] <- eval_A(i, j)
        return(u %*% eval_At_mat)
    }
    eval_At_times_u <- rir.annotateDepromised(eval_At_times_u)


    eval_AtA_times_u <- function(u) eval_At_times_u(eval_A_times_u(u))
    eval_AtA_times_u <- rir.annotateDepromised(eval_AtA_times_u)

    eval_A_cache <- matrix(0, n, n)
        for (i in 1:n)
            for (j in 1:n)
                eval_A_cache[[i, j]] <- 1/((i + j - 2) * (i + j - 1) / 2 + i)
    u <- rep(1, n)
    v <- rep(0, n)
    for (itr in seq(10)) {
        v <- eval_AtA_times_u(u)
        u <- eval_AtA_times_u(v)
    }

    cat(sqrt(sum(u * v) / sum(v * v)), "\n")
}
spectralnorm_alt_3 <- rir.annotateDepromised(spectralnorm_alt_3)


execute <- function(n) {
    spectralnorm_alt_3(n)
}
