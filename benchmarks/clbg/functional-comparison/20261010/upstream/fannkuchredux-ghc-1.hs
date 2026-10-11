{-  The Computer Language Benchmarks Game
    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
    contributed by Miha Vučkovič
-}

import System.Environment
import Control.Applicative

flop (2:x1:t) = x1:2:t
flop (3:x1:x2:t) = x2:x1:3:t
flop (4:x1:x2:x3:t) = x3:x2:x1:4:t
flop (5:x1:x2:x3:x4:t) = x4:x3:x2:x1:5:t
flop (6:x1:x2:x3:x4:x5:t) = x5:x4:x3:x2:x1:6:t
flop (7:x1:x2:x3:x4:x5:x6:t) = x6:x5:x4:x3:x2:x1:7:t

flop lst@(h:_) = r where
	(t, r) = flop' h (lst, t)
	flop' 0 (t, r) = (t, r)
	flop' n ((h:t), r) = flop' (n-1) (t, h:r)

flopS (1:_) = 0
flopS lst = 1 + flopS (flop lst)

rotate n (h:t) = rotate' (n-1) t where
	rotate' 0 l = h:l
	rotate' n (f:t) = f:(rotate' (n-1) t)

checksum i f
   | mod i 2 == 0 = f
   | True = -f

pfold r [] = r
pfold (ac, af) ((c, f):t)  = seq sc $ seq sf $ pfold (sc, sf) t where 
	sc = ac+c
	sf = max af f

permut n = foldr perm [[1..n]] [2..n] where
   perm x lst = concat [take x $ iterate (rotate x) l | l <- lst]

main = do
   n <- read.head <$> getArgs
   let (chksm, mflops) = pfold (0,0) $ map (\(i, p) -> let flops = flopS p in (checksum i flops, flops)) $ zip [0..] (permut n)
   putStrLn $ (show chksm) ++ "\nPfannkuchen(" ++ (show n) ++ ") = " ++ (show $ mflops)
