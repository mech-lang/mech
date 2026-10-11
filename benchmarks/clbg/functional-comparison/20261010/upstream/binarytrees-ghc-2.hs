--
-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Daniel Smith
--

import Data.Foldable (for_)
import Data.Monoid ((<>))
import System.Environment (getArgs)

main :: IO ()
main = do
    n <- read . head <$> getArgs

    let stretchN = n + 1
        stretchT = make stretchN
    putStrLn . makeOutput "stretch tree" stretchN $ check stretchT
    stretchT `seq` pure ()

    let longT = make n
        longC = check longT
    longC `seq` pure ()

    for_ [4, 6 .. n] $ \d -> do
        let c = 16 * 2 ^ (n - d)
        putStrLn . makeOutput (show c <> "\t trees") d $ sumT c d

    longT `seq` pure ()
    putStrLn $ makeOutput "long lived tree" n longC

makeOutput :: String -> Int -> Int -> String
makeOutput p n c = p <> " of depth " <> show n <> "\t check: " <> show c

data Tree = Nil | Node Tree Tree

sumT :: Int -> Int -> Int
sumT = go 0
  where
    go s 0 _ = s
    go s c d = s' `seq` t `seq` go s' (c - 1) d
      where
        t = make d
        s' = s + check t

check :: Tree -> Int
check Nil = 0
check (Node l r) = 1 + check l + check r

make :: Int -> Tree
make 0 = Node Nil Nil
make n = Node (make $ n - 1) (make $ n - 1)
