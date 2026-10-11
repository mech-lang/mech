-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Jaro Reinders

import qualified Data.Massiv.Array.Mutable as M
import qualified Data.Massiv.Array.Unsafe as U
import qualified Data.Massiv.Array as A
import System.Environment
import Data.Word
import Data.Char
import Control.Monad
import Control.Concurrent
import System.IO.Unsafe
import System.Posix.Internals

type Cumul = Word32
type MArray a = M.MArray M.RealWorld A.P Int a
type MStr = M.MArray M.RealWorld A.S Int Word8
type Str = A.Array A.P Int Word8
type Dist = A.Array A.P Int Cumul

bufferSize, linesPerBuffer, lineLength :: Int
bufferSize = linesPerBuffer * lineLength
linesPerBuffer = 3000
lineLength = 60

ia, ic, im :: Cumul
ia = 3877
ic = 29573
im = 139968
    
printRepeatedFasta :: Int -> Str -> IO ()
printRepeatedFasta n alu = do
  buf <- M.initializeNew Nothing (A.Sz m')
  let
    go 0 i j = do
      U.unsafeWrite buf i (fromIntegral (ord '\n'))
      go lineLength (i + 1) j
    go x i j
      | i < m' = do
        U.unsafeWrite buf i (U.unsafeIndex alu j)
        go (x - 1) (i + 1) ((j + 1) `rem` l)
      | otherwise = return ()
  go lineLength 0 0

  U.unsafeWithPtr (unsafePerformIO (U.unsafeFreeze A.Seq buf)) $ \ptr -> do
    replicateM_ q (c_write 1 ptr (fromIntegral m'))
    U.unsafeWrite buf (r + (r `quot` lineLength)) (fromIntegral (ord '\n'))
    void (c_write 1 ptr (fromIntegral (r + (r `quot` lineLength) + 1)))
  where
    l = A.unSz (A.size alu)
    m = lcm l lineLength
    m' = m + m `quot` lineLength
    (q,r) = quotRem n m

worker :: MVar (Int, Cumul, MVar ()) -> MVar () -> MArray Cumul -> MStr
  -> Str -> Dist -> IO ()
worker lock finish rnd out p0 p1 = loop where
  loop = do
    o' <- newEmptyMVar
    (n', s, o) <- takeMVar lock
    let n = min bufferSize n'
    s' <- if (n > 0) then prng n 0 s else return s
    putMVar lock (n' - n, s', o')
    when (n > 0) $ do
      select lineLength 0 0
      takeMVar o
      U.unsafeWithPtr out' $ \ptr -> c_write 1 ptr
        (fromIntegral (n + n `quot` lineLength))
      putMVar o' ()
      when (n' - n <= 0) (putMVar finish ())
      loop

  prng :: Int -> Int -> Cumul -> IO Cumul
  prng 0 _ s = return s
  prng todo n s = let s' = (ia * s + ic) `rem` im in do
    U.unsafeWrite rnd n s'
    prng (todo - 1) (n + 1) s'

  select :: Int -> Int -> Int -> IO ()
  select 0 outi rndi = do
    U.unsafeWrite out outi (fromIntegral (ord '\n'))
    select lineLength (outi + 1) rndi
  select l outi rndi
    | rndi < bufferSize = do
      rnd <- U.unsafeRead rnd rndi
      U.unsafeWrite out outi (lookupCumul rnd)
      select (l - 1) (outi + 1) (rndi + 1)
    | otherwise = return ()

  lookupCumul c = go 0 where
    go i
      | c <= U.unsafeIndex p1 i = U.unsafeIndex p0 i
      | otherwise = go (i + 1)

  out' = unsafePerformIO (U.unsafeFreeze A.Seq out)

printRandomFasta :: MVar (Int, Cumul, MVar ()) -> MVar () -> [MArray Cumul]
  -> [MStr] -> Str -> Dist -> Int -> IO ()
printRandomFasta lock finish rnds outs p0 p1 n = do
  (_,s,_) <- takeMVar lock
  o <- newMVar ()
  putMVar lock (n,s,o)
  mapM_ (\(rnd,out) -> forkIO (worker lock finish rnd out p0 p1))
    (zip rnds outs)
  takeMVar finish
  when (n `rem` lineLength /= 0) (putStrLn "")

main :: IO ()
main = do
  n <- readIO . head =<< getArgs

  U.unsafeWithPtr (A.fromList A.Seq $ map (fromIntegral . ord)
    ">ONE Homo sapiens alu\n") $ \ptr -> c_write 1 ptr 22
  printRepeatedFasta (2 * n) alu

  rndBufs <- replicateM workers
    (M.initializeNew Nothing (A.Sz bufferSize))
  outBufs <- replicateM workers
    (M.initializeNew Nothing (A.Sz (bufferSize + linesPerBuffer)))

  lock <- newMVar (undefined,42,undefined)
  finish <- newEmptyMVar

  U.unsafeWithPtr (A.fromList A.Seq $ map (fromIntegral . ord)
    ">TWO IUB ambiguity codes\n") $ \ptr -> c_write 1 ptr 25
  s <- printRandomFasta lock finish rndBufs outBufs p00 p01 (3 * n)

  U.unsafeWithPtr (A.fromList A.Seq $ map (fromIntegral . ord)
    ">THREE Homo sapiens frequency\n") $ \ptr -> c_write 1 ptr 30
  void $ printRandomFasta lock finish rndBufs outBufs p10 p11 (5 * n)
  where
    workers = 4
    
    alu = A.fromList A.Seq $ map (fromIntegral . ord)
      "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGT\
      \CAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGG\
      \CGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGG\
      \CGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAA\
      \AAA" 
    
    cumulativeProbabilities :: [(Char, Float)] -> (Str, Dist)
    cumulativeProbabilities xs = (A.fromList A.Seq as', A.fromList A.Seq bs')
      where
        as' = map (fromIntegral . ord) as
        bs' = init (map (floor . (* fromIntegral im)) (scanl1 (+) bs)) ++ [im]
        (as, bs) = unzip xs
    
    (p00,p01) = cumulativeProbabilities
      [('a', 0.27), ('c', 0.12), ('g', 0.12), ('t', 0.27), ('B', 0.02),
       ('D', 0.02), ('H', 0.02), ('K', 0.02), ('M', 0.02), ('N', 0.02),
       ('R', 0.02), ('S', 0.02), ('V', 0.02), ('W', 0.02), ('Y', 0.02)]
        
    (p10,p11) = cumulativeProbabilities
      [('a', 0.3029549426680), ('c', 0.1979883004921),
       ('g', 0.1975473066391), ('t', 0.3015094502008)]
