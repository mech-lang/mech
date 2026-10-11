--
-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Don Stewart
-- Basic parallelization by Roman Kashitsyn and Artem Pelenitsyn
-- Tail call optimizations by Izaak Weiss
--

import System.Environment
import Data.Bits
import Text.Printf
import Control.Parallel.Strategies

--
-- an artificially strict tree. 
--
-- normally you would ensure the branches are lazy, but this benchmark
-- requires strict allocation.
--
data Tree = Nil | Node !Tree !Tree

minN = 4

io s n t = printf "%s of depth %d\t check: %d\n" s n t

main = do
    n <- getArgs >>= readIO . head
    let maxN     = max (minN + 2) n
        stretchN = maxN + 1

    -- stretch memory tree
    let c = checkPar4 (makePar4 stretchN)
    io "stretch tree" stretchN c

    -- allocate a long lived tree
    let !long    = makePar2 maxN

    -- allocate, walk, and deallocate many bottom-up binary trees
    let vs = (depth minN maxN) `using`
             (parList $ evalTuple3 r0 r0 rseq)
    mapM_ (\((m,d,i)) -> io (show m ++ "\t trees") d i) vs

    -- confirm the the long-lived binary tree still exists
    io "long lived tree" maxN (check long)

-- generate trees of depth @d@, @d+1@... until @m@
depth :: Int -> Int -> [(Int, Int, Int)]
depth d m
    | d <= m    = (n, d, sumT d n 0) : depth (d+2) m
    | otherwise = []
  where n = 1 `shiftL` (m - d + minN)

-- allocate and check @i@ trees of depth @d@
sumT :: Int -> Int -> Int -> Int
sumT d 0 t = t
sumT d i t = sumT d (i-1) (t + a)
  where a = check (make d)

-- traverse the tree, counting up the nodes
check :: Tree -> Int
check t = tailCheck t 0

tailCheck :: Tree -> Int -> Int
tailCheck Nil        !a = a
tailCheck (Node l r) !a = tailCheck l $ tailCheck r $ a + 1

-- traverse and count nodes in parallel (4-threaded version)
checkPar4 :: Tree -> Int
checkPar4 (Node (Node ll lr) (Node rl rr)) = all + alr + arl + arr + 3 `using` strat where
  all = tailCheck ll 0
  alr = tailCheck lr 0
  arl = tailCheck rl 0
  arr = tailCheck rr 0
  strat v = do
    rpar all
    rpar alr
    rpar arl
    rseq arr
    return v

-- build a tree
make :: Int -> Tree
make d =
  if d < 10 then make' d d else makePar2 d

-- This function has an extra argument to suppress the
-- Common Sub-expression Elimination optimization
make' :: Int -> Int -> Tree
make' _  0 = Node Nil Nil
make' !n d = Node (make' (n - 1) (d - 1)) (make' (n + 1) (d - 1))

-- build a tree in parallel (4-threaded version)
makePar4 :: Int -> Tree
makePar4 d = Node (Node ll lr) (Node rl rr) `using` strat where
  !d' = d - 2
  ll = make' 0 d'
  lr = make' 1 d'
  rl = make' 2 d'
  rr = make' 3 d'
  strat v = do
    rpar ll
    rpar lr
    rpar rl
    rseq rr
    return v

-- build a tree in parallel (2-threaded version)
makePar2 :: Int -> Tree
makePar2 d = Node l r  `using` strat where
  !d' = d - 1
  l = make' 0 d'
  r = make' 1 d'
  strat v = do
    rpar l
    rseq r
    return v
