-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
-- contributed by Bryan O'Sullivan
-- modified by Eugene Kirpichov: pidgits only generates
-- the result string instead of printing it. For some
-- reason, this gives a speedup.
-- modified by W. Gordon Goodsman to do two divisions
-- as per the new task requirements.
-- further modified by W. Gordon Goodsman to reduce the memory leak
-- caused by excess laziness in computing the output string before output.

import System.Environment

pidgits n = (1,9) % (0 # (1,0,1)) where
 (i,k)%(d:ds')
  | i <= n = putStr (show d ++
               (if k<=0 then "\t:" ++ show i ++ "\n" else "")) >> (j,m)%ds'
  | True = if k<9 then putStrLn (replicate k ' ' ++ "\t:" ++ show n)
           else putStr ""
  where j = i + 1; m = if k<=0 then 9 else k - 1
 j # s | n>a || q/=r = k # t
       | True = q : k # (n*10,(a-(q*d))*10,d) -- inline eliminateDigit
  where k = j+1; t@(n,a,d)=k&s; q=3$t; r=4$t -- two calls to extractDigit
 c$(n,a,d) = (c*n+a)`div`d -- extractDigit
 j&(n,a,d) = (n*j,(a+n*2)*y,d*y) where y=(j*2+1) -- nextDigit

main = pidgits.read.head =<< getArgs
