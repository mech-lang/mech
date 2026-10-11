-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
-- contributed by Branimir Maksimovic

{- original Haskell implementation from spigot.pdf document
 - I've just added printPi and main, also replaced floor and / with div
 - because for some reason div is much faster
 -}

module Main where
import System.Environment
main = do [n] <- getArgs
          printPi $ take (read n) Main.pi

printPi digits = printPi' digits 10 0
  where printPi' [] ndigs sum' = do mapM_ (\_ -> putChar ' ') [1..ndigs]
                                    putStr $ "\t:" ++ show sum' ++ "\n"
        printPi' xxs 0 sum' = do putStr $ "\t:" ++ show sum' ++ "\n"
                                 printPi' xxs 10 sum'
        printPi' (x:xs) ndigs sum' = do putStr $ show x
                                        printPi' xs (ndigs-1) (sum'+1)

stream :: (b->c) -> (b->c->Bool) -> (b->c->b) -> (b->a->b) -> b -> [a] -> [c] 
stream next safe prod cons z (x:xs) 
  = if safe z y 
       then y : stream next safe prod cons (prod z y) (x:xs) 
       else stream next safe prod cons (cons z x) xs 
  where y = next z

type LFT = (Integer, Integer, Integer, Integer) 
floorExtr :: LFT -> Integer -> Integer
floorExtr (q,r,s,t) x = ((fromInteger q) * fromInteger x + (fromInteger r)) `div`
                        ((fromInteger s) * fromInteger x + (fromInteger t)) 
unit :: LFT 
unit = (1,0,0,1) 
comp :: LFT -> LFT -> LFT 
comp (q,r,s,t) (u,v,w,x) = (q*u+r*w,q*v+r*x,s*u+t*w,s*v+t*x)

pi = stream next safe prod cons init lfts 
  where
        init = unit 
        lfts = [(k, 4*k+2, 0, 2*k+1) | k<-[1..]] 
        next z = floorExtr z 3
        safe z n = (n == floorExtr z 4) 
        prod z n = comp (10, -10*n, 0, 1) z 
        cons z z'  = comp z z'
