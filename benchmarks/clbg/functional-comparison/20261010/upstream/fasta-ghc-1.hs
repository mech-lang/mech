-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Roman Kashitsyn

import qualified Data.ByteString.Char8 as BS
import           System.Environment    (getArgs)
import           System.IO             (stdout)

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

printRepeatedFasta :: BS.ByteString -> Int -> IO ()
printRepeatedFasta s = go lineWidth n
  where
    !n = BS.length s
    go 0 sn left = BS.putStrLn "" >> go lineWidth sn left
    go w  0 left = go w n left
    go w  _ 0    = if w == lineWidth then return () else BS.putStrLn ""
    go w sn left = do
      let toTake = w `min` sn `min` left
      BS.putStr (BS.take toTake $ BS.drop (n - sn) s)
      go (w - toTake) (sn - toTake) (left - toTake)

printRandomFasta :: DistF -> Int -> Int -> IO Int
printRandomFasta dist seed n = go n seed
  where
    genChar seed = Just (dist f, seed')
      where !seed' = nextSeed seed
            !f = fromIntegral seed' / (fromIntegral modulo)

    go 0     !seed = return seed
    go total !seed = do
      let toTake = total `min` lineWidth
          (!b, Just seed') = BS.unfoldrN toTake genChar seed
      BS.putStrLn b
      go (total - toTake) seed'

main :: IO ()
main = do
  n <- getArgs >>= readIO . head

  BS.putStrLn ">ONE Homo sapiens alu"
  printRepeatedFasta alu (2 * n)

  BS.putStrLn ">TWO IUB ambiguity codes"
  seed' <- printRandomFasta iubF 42 (3 * n)

  BS.putStrLn ">THREE Homo sapiens frequency"
  printRandomFasta homoF seed' (5 * n)
  return ()
