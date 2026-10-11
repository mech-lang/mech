probabilities <- c(0.302954942668,0.1979883004921,0.1975473066391,0.3015094502008)
cdf <- cumsum(probabilities)
random <- (0:139967)/139968
cat(findInterval(random,cdf),sep="\n")
