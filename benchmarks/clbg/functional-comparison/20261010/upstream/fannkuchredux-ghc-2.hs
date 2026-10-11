{-  The Computer Language Benchmarks Game

    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

    contributed by Miha Vučkovič

-}

import System.Environment
import qualified Data.Vector.Unboxed as V
import qualified Data.Vector.Unboxed.Mutable as MV
import qualified Data.Vector.Generic.Mutable as GMV
import Control.Monad (unless, void, forM_)
import Control.Monad.ST
import Control.Parallel
import Control.Parallel.Strategies
import Data.STRef

flopST v flopCount = do
    h <- MV.unsafeRead v 0
    if h == 1 then return flopCount
    else do
        GMV.reverse $ MV.unsafeSlice 0 h v
        flopST v (flopCount + 1)

countFlops :: V.Vector Int -> Int
countFlops v = runST $ do
    mv <- V.thaw v
    flopST mv 0

permut n = foldr permConcat [V.fromList [1..n]] [5..n]

permConcat x lst = concat [take x $ iterate (rotate x) l | l <- lst]

perm :: Int -> V.Vector Int -> [V.Vector Int]
perm x l = take x $ iterate (rotate x) l

rotate :: Int -> V.Vector Int -> V.Vector Int
rotate 1 xs = xs
rotate n v = runST $ do
    mv <- V.thaw v
    veryUnsafeV <- V.unsafeThaw v
    h <- MV.unsafeRead mv 0
    MV.unsafeCopy (MV.unsafeSlice 0 (n-1) mv) (MV.unsafeSlice 1 (n-1) veryUnsafeV)
    MV.unsafeWrite mv (n-1) h
    V.unsafeFreeze mv

calculateMaxAndChecksum :: [Int] -> (Int, Int)
calculateMaxAndChecksum = go 0 0
    where go !m !c [] = (m, c)
          go !m !c [x] = (max m x, c + x)
          go !m !c (x0:x1:xs) = go (max3 m x0 x1) (c + x0 - x1) xs
          max3 !a !b !c = max c (max a b)

niceParallelMap :: NFData b => Int -> (a -> b) -> [a] -> [b]
niceParallelMap bufferSize f xs = runEval $ parBuffer bufferSize rdeepseq $ map f xs

main :: IO ()
main = do
   n <- fmap (read . head) getArgs
   let permutations = permut n
       flopCounts = concat $ niceParallelMap 50
            (map countFlops . concatMap (perm 2) . concatMap (perm 3) . perm 4) permutations
       (mflops, checksum) = calculateMaxAndChecksum flopCounts
   putStrLn $ show checksum ++ "\nPfannkuchen(" ++ show n ++ ") = " ++ show mflops
