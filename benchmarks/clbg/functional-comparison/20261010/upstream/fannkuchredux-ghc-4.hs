{-  The Computer Language Benchmarks Game

    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

    contributed by Branimir Maksimovic
    optimized/rewritten by Bryan O'Sullivan
    modified by Gabriel Gonzalez
    fix for GHC 9.2 by Artem Pelenitsyn    
-}

import System.Environment
import Text.Printf
import Data.Bits

import qualified Data.Vector.Unboxed.Mutable as VM
import qualified Data.Vector.Generic.Mutable as VG
import qualified Data.Vector.Unboxed as V

main = do
    n <- getArgs >>= readIO.head
    (checksum,maxflips) <- fannkuch n
    printf "%d\nPfannkuchen(%d) = %d\n" checksum n maxflips

fannkuch :: Int -> IO (Int, Int)
fannkuch n = do
    perm <- V.unsafeThaw $ V.enumFromN 1 n
    !tperm <- VG.new n
    !cnt <- VG.replicate n 0
    let
        loop :: Int -> Int -> Int -> IO(Int,Int)
        loop !c !m !pc = do
            b <- next_permutation perm n cnt
            if b == False
            then return (c,m)
            else do
                VM.unsafeCopy tperm perm
                let count_flips !flips = {-# SCC "count_flips" #-} do
                        f <- VM.unsafeRead tperm 0
                        if f == 1
                        then loop (c + (if pc .&. 1 == 0 then flips else -flips))
                                (max m flips)
                                (pc+1)
                        else do
                                VG.reverse $ VM.unsafeSlice 0 f tperm
                                count_flips (flips+1)
                count_flips 0
    loop 0 0 1

next_permutation :: VM.IOVector Int -> Int -> VM.IOVector Int -> IO Bool
next_permutation perm !n !cnt = loop 1
    where
    loop :: Int -> IO Bool
    loop i
        | i >= n = done i
        | otherwise = do
            tmp <- VM.unsafeRead perm 0
            let
                rotate :: Int -> IO()
                rotate j
                    | j >= i = VM.unsafeWrite perm i tmp
                    | otherwise = do
                        !v <- VM.unsafeRead perm (j+1)
                        VM.unsafeWrite perm j v
                        rotate (j+1)
            rotate 0
            v <- VM.unsafeRead cnt i
            if v >= i
            then VM.unsafeWrite cnt i 0 >> loop (i+1)
            else done i

    done :: Int -> IO Bool
    done i
        | i >= n = return False
        | otherwise = do
            v <- VM.unsafeRead cnt i
            VM.unsafeWrite cnt i (v+1)
            return True
