-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

-- contributed by Greg Buchholz
-- fix overflowing Chars by switching to Word8
--   and use vectors/bytestrings instead of lists -- Artem Pelenitsyn


import qualified Data.Vector.Unboxed as V
import qualified Data.ByteString as B
import qualified Data.ByteString.Builder as B
import Data.Complex
import Data.Bool
import Data.Word
import System.Environment
import System.IO (stdout)

type C = Complex Double
type V = V.Vector
f = fromIntegral -- conversions b/w numeric types

main = do   [arg] <- getArgs
            let width = read arg
            let pts = points width
            putStr $ "P4\n" ++ arg ++ " " ++ arg ++ "\n"
            B.hPutBuilder stdout $ makePBM 0 0 (fractal pts)

limit  = 4    -- max magnitude squared
iter   = 50+1 -- add one to compensate for the 'iterate' function

points :: Int -> V C
points w = V.generate (w * w) gen -- generate vector of length width^2
  where
    gen i = (2*(f x)/(f w) - 1.5) :+ (2*(f y)/(f w) - 1)
      where (y, x) = quotRem i w

mandel :: C -> C -> C
mandel c z = z * z + c

fractal :: V C -> V Bool -- for every point check if it goes to "infinity"
fractal = V.map ((\f-> iter == length (takeIter (iterate f (0:+0)))) . mandel)
        where takeIter a = take iter (takeWhile (\x-> magnitudeSq x < limit) a)

magnitudeSq :: C -> Double
magnitudeSq (x :+ y) = x*x + y*y

makePBM :: Int -> Word8 -> V Bool -> B.Builder
makePBM i acc v | Just (x, xs) <- V.uncons v = case i of
                    8 -> B.word8 (fromIntegral acc) <> makePBM 0 0 v
                    _ -> makePBM (i+1) (bool (acc*2) (acc*2+1) x) xs
                | otherwise = B.word8 (acc * 2^(8-i))
