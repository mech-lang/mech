{-
The Computer Language Benchmarks Game

https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

Contributed by Branimir Maksimovic.
Optimized/rewritten by Bryan O'Sullivan.
Parallelized and rewritten by James Brock.

Build:
ghc --make -fllvm -O2 -threaded -XBangPatterns -XScopedTypeVariables -rtsopts fannkuch-redux.hs -o fannkuch-redux

Run 1-Core:
fannkuch-redux +RTS -N1 -RTS 12

Run 4-Core:
fannkuch-redux +RTS -N4 -RTS 12
-}

module Main(main) where

import System.Environment
import Data.Bits
import Data.List
import Control.Monad
import Control.Concurrent
import Control.Concurrent.Async

import qualified Data.Vector.Unboxed.Mutable as VM
import qualified Data.Vector.Generic.Mutable as VG


main :: IO ()
main = do
    n <- readIO . head =<< getArgs

    (maxFlipsCount, checkSum) <- fannkuch n

    putStr $ unlines
        [ show checkSum
        , "Pfannkuchen(" ++ show n ++ ") = " ++ show maxFlipsCount
        ]


fannkuch
    :: Int
        -- ^ n, the size of the fannkuch-redux problem.
    -> IO (Int, Int)
        -- ^ (max flips count, checksum)
fannkuch !n = do

    -- Number of permutations to consider.
    let numPermutations = factorial n

    -- Number of cores available for work.
    numCapabilities <- getNumCapabilities

    -- The amount of work which we would like to give to each core.
    let workSize = max 1000 $ numPermutations `div` numCapabilities

    -- Divide up the permutations into workSize units
    let workBoundary = takeWhile (<=numPermutations) $ iterate (+workSize) 1

    -- Upper and lower permutation index bounds for each workSize unit
    let workRanges = zip workBoundary $ tail workBoundary ++ [numPermutations+1]

    -- Perform the work.
    results <- mapConcurrently (uncurry $ work n) workRanges

    -- Gather up the results and return.
    return $ foldl1' (\(fc0,cs0) (fc1,cs1) -> (max fc0 fc1, cs0+cs1)) results


-- | Basic tail-call factorial. Never called on the hot path of this program.
factorial :: Int -> Int
factorial z0 = go z0 1
  where
    go 1 !answer = answer
    go !z !answer = go (z-1) (answer*z)


-- | Work function which counts flips and calculates checksum for a range of
-- permutations.
work
    :: Int
        -- ^ n, The size of the fannkuch-redux problem.
    -> Int
        -- ^ Lower bound inclusive of the permutation indices for this work.
    -> Int
        -- ^ Upper bound exclusive of the permutation indices for this work.
    -> IO (Int, Int)
        -- ^ (max flips count, checksum)
work !n !permIndexBegin !permIndexEnd = do

    -- Allocate mutable vector memory in the worker thread. Hopefully no two
    -- threads will allocate mutable vectors which share a cache line.

    -- Permutation vector.
    perm  :: VM.IOVector Int <- VM.unsafeNew n

    -- Working temporary permutation vector.
    tperm :: VM.IOVector Int <- VM.unsafeNew n

    -- Count vector.
    --
    -- > To optimize the process I use an intermediate data structure,
    -- > count[], which keeps count of how many rotations have been done
    -- > at every level.
    count :: VM.IOVector Int <- VM.unsafeNew n

    -- Initialize the perm and count vectors.
    permIndex n permIndexBegin perm count

    -- Preserve these mutually-recursive loop and count_flips functions
    -- from fannkuch-redux Haskell GHC #4 because they seem unimprovably fast.
    let
        loop
            :: Int -- c,  checksum
            -> Int -- m,  flip count
            -> Int -- pc, permutation index
            -> IO(Int,Int)
        loop !c !m !pc
            | pc == (permIndexEnd-1) = return (m, c)
            | otherwise = do
                permNext n perm count
                VM.unsafeCopy tperm perm
                let count_flips :: Int -> IO (Int, Int)
                    count_flips !flips = {-# SCC "count_flips" #-} do
                        f <- VM.unsafeRead tperm 0
                        if f == 1
                        then loop
                                (c + (if pc .&. 1 == 0 then flips else -flips))
                                (max m flips)
                                (pc+1)
                        else do
                            VG.reverse $ VM.unsafeSlice 0 f tperm
                            count_flips (flips+1)
                count_flips 0
    loop 0 0 (permIndexBegin-1)


-- | Generate the next permutation vector and count vector.
permNext
    :: Int -- ^ Permutation length.
    -> VM.IOVector Int -- ^ Vector to be mutated to next permutation.
    -> VM.IOVector Int -- ^ count vector for recursion depth state.
    -> IO ()
permNext !n !perm !count = go 1
  where
    go
        :: Int -- ^ i loops over [1..n-1]
        -> IO ()
    go !i
        | i >= n    = return ()
        | otherwise = do

            -- left-rotate the first i+1 places of perm
            rotateLeft perm $ i+1

            counti <- VM.unsafeRead count i
            if counti >= i
            then do
                VM.unsafeWrite count i 0
                go $ i+1
            else do
                VM.unsafeWrite count i $ counti+1
                return ()


-- | From a permutation index, generate permutation vector and count vector.
--
-- > It should be clear now how to generate a permutation and corresponding
-- > count[] array from an arbitrary index. Basically,
-- >
-- > count[k] = ( index % (k+1)! ) / k!
-- >
-- > is the number of rotations we need to perform on elements 0..k.
-- > Doing it in the descending order from n-1 to 1 gives us both the count[]
-- > array and the permutation.
permIndex
    :: Int -- ^ Permutation length.
    -> Int -- ^ ith permutation index, 1-based.
    -> VM.IOVector Int -- ^ Mutable permutation vector output.
    -> VM.IOVector Int -- ^ Mutable count vector output.
    -> IO ()
permIndex !n !i !perm !count = do

    -- initialize perm to [1,2,..n]
    forM_ [0..n-1] (\k -> VM.unsafeWrite perm k $ k + 1)

    VM.unsafeWrite count 0 0 -- count[0] is always 0.

    -- forM_ k = [n-1..1] over the count vector
    forM_ (take (n-1) $ iterate (subtract 1) (n-1)) $ \ k -> do
        let countk = (i `mod` factorial (k+1)) `div` factorial k
        VM.unsafeWrite count k countk
        replicateM_ countk $ rotateLeft perm $ k+1


-- | Left-rotate the first i places of perm where i >= 2.
rotateLeft
    :: VM.IOVector Int
    -> Int -- ^ Precondition: i >= 2
    -> IO ()
rotateLeft !perm !i = do
    !perm0 <- VM.unsafeRead perm 0
    VM.unsafeMove (VM.unsafeSlice 0 (i-1) perm) (VM.unsafeSlice 1 (i-1) perm)
    VM.unsafeWrite perm (i-1) perm0
