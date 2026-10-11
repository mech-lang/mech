{-  The Computer Language Benchmarks Game

    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

    contributed by Branimir Maksimovic
    optimized/rewritten by Bryan O'Sullivan
    modified by Dmitry Ivanov
-}

import Control.Monad (replicateM_)
import Control.Monad.ST
import Control.Parallel.Strategies
import Data.Bits ((.&.))
import Data.List (foldl')

import qualified Data.Vector.Unboxed.Mutable as VM
import qualified Data.Vector.Generic.Mutable as VG
import qualified Data.Vector.Unboxed as V

import System.Environment
import Text.Printf

main :: IO ()
main = do
    n <- fmap (read . head) getArgs
    let (checksum, maxflips) = reduce $ parMap rdeepseq (fannkuch n) [0 .. (n - 1)]
    printf "%d\nPfannkuchen(%d) = %d\n" checksum n maxflips

reduce :: [(Int, Int)] -> (Int, Int)
reduce = foldl' (\(!c1, !f1) (!c2, !f2) -> (c1 + c2, max f1 f2)) (0, 0)

rotate mv = do
    !h <- VM.unsafeRead mv 0
    VM.unsafeMove (VM.unsafeInit mv) (VM.unsafeTail mv)
    VM.unsafeWrite mv (VM.length mv - 1) h

fannkuch :: Int -> Int -> (Int, Int)
fannkuch n i = runST $ do
    !perm <- V.unsafeThaw $ V.enumFromN 1 n
    replicateM_ i $ rotate perm
    !tperm <- VG.new n
    !cnt <- VG.replicate n 0
    let loop !c !m !countdown = do
            next_permutation perm n cnt
            if countdown == 0
            then return (c, m)
            else do
                VM.unsafeCopy tperm perm
                let count_flips !flips = do
                        f <- VM.unsafeRead tperm 0
                        if f == 1
                        then loop (c + if countdown .&. 1 == 1 then flips else -flips)
                                    (max m flips)
                                    (pred countdown)
                        else do
                            VG.reverse $ VM.unsafeSlice 0 f tperm
                            count_flips (flips + 1)
                count_flips 0
    loop 0 0 (product [1 .. n - 1])

next_permutation !perm !n !cnt = loop 1
    where
    loop !i
        | i >= n = done i
        | otherwise = do
            tmp <- VM.unsafeRead perm 0
            let rotate' j
                    | j >= i = VM.unsafeWrite perm i tmp
                    | otherwise = do
                        !v <- VM.unsafeRead perm (j+1)
                        VM.unsafeWrite perm j v
                        rotate' (j+1)
            rotate' 0
            v <- VM.unsafeRead cnt i
            if v >= i
            then VM.unsafeWrite cnt i 0 >> loop (i+1)
            else done i
    done !i
        | i >= n = return ()
        | otherwise = do
            v <- VM.unsafeRead cnt i
            VM.unsafeWrite cnt i (v+1)
