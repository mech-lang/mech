-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Roman Kashitsyn
-- Optimizations by Vassil Keremidchiev

import qualified Data.ByteString.Char8 as BS
import           System.Environment    (getArgs)
import           System.IO             (stdout, hSetBuffering, BufferMode(..))
import           Data.ByteString.Builder

alu = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGG\
    \TCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGG\
    \CGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGC\
    \GGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"

type DistF = Double -> Char

-- These cumulative distribution functions are not very pretty, but they
-- give ~10% speedup in execution time comparing to a list of pairs.
iubF, homoF :: DistF
iubF f
  | f < 0.27 = 'a'
  | f < 0.39 = 'c'
  | f < 0.51 = 'g'
  | f < 0.78 = 't'
  | f < 0.80 = 'B'
  | f < 0.82 = 'D'
  | f < 0.84 = 'H'
  | f < 0.86 = 'K'
  | f < 0.88 = 'M'
  | f < 0.90 = 'N'
  | f < 0.92 = 'R'
  | f < 0.94 = 'S'
  | f < 0.96 = 'V'
  | f < 0.98 = 'W'
  | otherwise = 'Y'

homoF f
  | f < 0.302954942668  = 'a'
  | f < 0.5009432431601 = 'c'
  | f < 0.6984905497992 = 'g'
  | otherwise = 't'

lineWidth, modulo :: Int
lineWidth = 60
modulo = 139968


nextSeed :: Int -> Int
nextSeed seed = (3877 * seed + 29573) `rem` modulo

printRepeatedFasta :: BS.ByteString -> Int -> Builder
printRepeatedFasta s x = go lineWidth n x mempty
  where
    !n = BS.length s
    nl = char8 '\n'

    go 0 sn left before = go lineWidth sn left (before <> nl)
    go w 0  left before = go w n left before
    go w _  0    before = if w == lineWidth then before else before <> nl
    go w sn left before = go (w - toTake) (sn - toTake) (left - toTake) (before <> (byteString newone)) where
      toTake = w `min` sn `min` left
      newone = BS.take toTake $ BS.drop (n - sn) s

printRandomFasta :: DistF -> Int -> Int -> (Builder, Int)
printRandomFasta dist seed n = go n seed mempty
  where
    nl = char8 '\n'
    genChar seed = Just (dist f, seed')
      where !seed' = nextSeed seed
            !f = fromIntegral seed' / (fromIntegral modulo)

    go 0     !seed before = (before, seed)
    go total !seed before = go (total - toTake) seed' (before <> (byteString b) <> nl) where
      toTake = total `min` lineWidth
      (!b, Just seed') = BS.unfoldrN toTake genChar seed

main :: IO ()
main = do
  n <- getArgs >>= readIO . head

  let a = ">ONE Homo sapiens alu\n"
      res = printRepeatedFasta alu (2 * n)
      
      b = ">TWO IUB ambiguity codes\n"
      (res', seed') = printRandomFasta iubF 42 (3 * n)

      c = ">THREE Homo sapiens frequency\n"
      (res'', _) = printRandomFasta homoF seed' (5 * n)

  hPutBuilder stdout $ a <> res <> b <> res' <> c <> res''
