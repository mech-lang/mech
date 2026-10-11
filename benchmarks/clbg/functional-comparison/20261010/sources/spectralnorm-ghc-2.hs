--
-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
-- Contributed by Greg Buchholz 

import Data.Array.Base
import System.Environment
import Numeric

main = do 
        [arg] <- getArgs
        let n = (read arg) - 1 
        let init = listArray (0,n) (repeat 1.0)
        let (v:u:rest) = drop 19 $ iterate (eval_AtA_times_u n) init
        let vBv = sum [(u!i)*(v!i) |i<-[0..n]]
        let vv  = sum [(v!i)*(v!i) |i<-[0..n]]
        putStrLn $ showFFloat (Just 9) (sqrt (vBv/vv)) ""

eval_AtA_times_u n u = eval_At_times_u n v
    where v = eval_A_times_u n u

eval_A x y = 1.0/((i+j)*(i+j+1)/2+i+1)
    where i = fromIntegral x
          j = fromIntegral y

eval_A_times_u :: Int -> UArray Int Double -> UArray Int Double
eval_A_times_u n u = unsafeAccumArray (+) 0 (0,n) 
                     [(i,(eval_A i j) * u!j)|i<-[0..n], j<-[0..n]]
   
eval_At_times_u :: Int -> UArray Int Double -> UArray Int Double
eval_At_times_u n u = unsafeAccumArray (+) 0 (0,n) 
                      [(i,(eval_A j i) * u!j)|i<-[0..n], j<-[0..n]]
