probabilities <- c(0.27,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02)
cdf <- cumsum(probabilities)
random <- (0:139967)/139968
cat(findInterval(random,cdf),sep="\n")
