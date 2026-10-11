# Canonical ten-pair application in base R; inspired by Leo Osvald's R shootout.
run <- function(n) {
  p <- matrix(c(0,0,0,4.84143144246472090,-1.16032004402742839,-0.103622044471123109,8.34336671824457987,4.12479856412430479,-0.403523417114321381,12.8943695621391310,-15.1111514016986312,-0.223307578892655734,15.3796971148509165,-25.9193146099879641,0.179258772950371181),5,3,byrow=TRUE)
  v <- matrix(c(0,0,0,1.66007664274403694e-3,7.69901118419740425e-3,-6.90460016972063023e-5,-2.76742510726862411e-3,4.99852801234917238e-3,2.30417297573763929e-5,2.96460137564761618e-3,2.37847173959480950e-3,-2.96589568540237556e-5,2.68067772490389322e-3,1.62824170038242295e-3,-9.51592254519715870e-5),5,3,byrow=TRUE) * 365.24
  m <- c(1,9.54791938424326609e-4,2.85885980666130812e-4,4.36624404335156298e-5,5.15138902046611451e-5) * (4*pi*pi)
  pair <- combn(5L,2L)
  incidence <- outer(seq_len(5L),pair[1L,],`==`) - outer(seq_len(5L),pair[2L,],`==`)
  v[1L,] <- -colSums(v*m)/m[1L]
  energy <- function() sum(m*rowSums(v*v)/2) - sum(m[pair[1L,]]*m[pair[2L,]]/sqrt(rowSums((p[pair[1L,],]-p[pair[2L,],])^2)))
  cat(sprintf("%.9f\n",energy()))
  for (step in seq_len(n)) {
    delta <- p[pair[1L,],] - p[pair[2L,],]
    distance <- sqrt(rowSums(delta*delta))
    force <- delta*(0.01/(distance*distance*distance))*m[pair[1L,]]*m[pair[2L,]]
    v <- v - (incidence %*% force)/m
    p <- p + 0.01*v
  }
  cat(sprintf("%.9f\n",energy()))
}
run(as.integer(commandArgs(TRUE)[1L]))
