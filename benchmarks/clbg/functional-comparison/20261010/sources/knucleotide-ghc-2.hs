-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by cahu ette


module Main where

import Data.Bits
import Data.List
import Data.Word
import Data.Hashable
import Data.Traversable
import Text.Printf

import Data.STRef
import Control.Monad
import Control.Monad.ST
import Control.Parallel.Strategies


import qualified Data.Map.Strict           as M
import qualified Data.HashTable.Class      as HC
import qualified Data.HashTable.ST.Basic   as H
import qualified Data.ByteString.Char8     as B


type HashTable s k v = H.HashTable s k v


{- By using only 2 bits to encode keys, it's important to use a different table
 - for different key sizes. Otherwise, if we encode 'A' as 0x00, "AT" and
 - "AAT" would map to the same bucket in the table.
 -
 - We could use 3 bits per letter to avoid this problem if needed.
 -}
bitsForChar :: Char -> Word64
bitsForChar 'a' = 0
bitsForChar 'A' = 0
bitsForChar 'c' = 1
bitsForChar 'C' = 1
bitsForChar 'g' = 2
bitsForChar 'G' = 2
bitsForChar 't' = 3
bitsForChar 'T' = 3
bitsForChar _   = error "Ay, Caramba!"


charForBits :: Word64 -> Char
charForBits 0 = 'A'
charForBits 1 = 'C'
charForBits 2 = 'G'
charForBits 3 = 'T'
charForBits _ = error "Ay, Caramba!"


packKey :: B.ByteString -> Word64
packKey = go zeroBits
  where
    go k bs = case B.uncons bs of
        Nothing      -> k
        Just (c, cs) -> go (unsafeShiftL k 2 .|. bitsForChar c) cs
{-# INLINE packKey #-}

unpackKey :: Int -> Word64 -> B.ByteString
unpackKey = go []
  where
    go s 0 _ = B.pack s
    go s l i = go (charForBits (i .&. 3) : s) (l-1) (unsafeShiftR i 2)
{-# INLINE unpackKey #-}


updateTable :: (Eq k, Hashable k)
            => HashTable s k (STRef s Int)
            -> (Int -> Int)
            -> k
            -> ST s ()
updateTable t f k = do
    mv <- H.lookup t k
    case mv of
        Nothing -> newSTRef 1 >>= H.insert t k
        Just v  -> modifySTRef' v f
{-# INLINE updateTable #-}


getVal :: (Eq k, Hashable k)
       => HashTable s k (STRef s Int)
       -> k
       -> ST s Int
getVal t k = do
    mv <- H.lookup t k
    case mv of Nothing -> return 0
               Just sv -> readSTRef sv
--{-# INLINE getVal #-}


tableToList :: HashTable s k (STRef s a) -> ST s [(k, a)]
tableToList t = do
    pairs <- HC.toList t
    forM pairs $ \(k, v) -> do
        a <- readSTRef v
        return (k, a)


countOccurrences :: Int -> Int -> B.ByteString -> ST s (HashTable s Word64 (STRef s Int))
countOccurrences jumpSize frameSize input = do
    t <- H.new

    let go bs = unless (B.length bs < frameSize) $ do
            let k = takeFrame bs
            updateTable t (+1) (packKey k)
            go (nextFrame bs)

    go input
    return t

  where
    takeFrame = B.take frameSize
    nextFrame = B.drop jumpSize


extractSequence :: String -> B.ByteString -> B.ByteString
extractSequence s = findSeq
  where
    prefix = B.pack ('>' : s)
    skipSeq =
          B.dropWhile (/= '>')
        . B.drop 1
    takeSeq =
          B.filter    (/= '\n')
        . B.takeWhile (/=  '>') -- extract until next header
        . B.dropWhile (/= '\n') -- skip header
    findSeq str
        | prefix `B.isPrefixOf` str  =  takeSeq str
        | otherwise                  =  findSeq (skipSeq str)



main :: IO ()
main = do
    s <- extractSequence "THREE" <$> B.getContents

    let keys    = ["GGT","GGTA","GGTATT","GGTATTTTAATT","GGTATTTTAATTTATAGT"]
    let threads = [0 .. 63]

    let threadWorkOcc key tid = runST $ do
            t <- countOccurrences (length threads) (B.length key) (B.drop tid s)
            getVal t (packKey key)

    let calcOcc key = sum $ runEval $
            mapM (rpar . threadWorkOcc (B.pack key)) threads

    let threadWorkFreq len tid = runST $ do
            t  <- countOccurrences (length threads) len (B.drop tid s)
            vs <- tableToList t
            return $ map (\(k, v) -> (B.unpack (unpackKey len k), freq v)) vs
          where
            freq v = 100 * fromIntegral v / fromIntegral (B.length s - len + 1)

    let calcFreq len =
            let l = concat $ runEval $ mapM (rpar . threadWorkFreq len) threads
                m = foldr (uncurry $ M.insertWith (+)) M.empty l
            in
                M.toList m

    let resultsOcc = map (\k -> (k, calcOcc k)) keys

    printFreq (calcFreq 1)
    putStrLn ""
    printFreq (calcFreq 2)
    putStrLn ""
    forM_ resultsOcc $ \(k, r) -> printf "%d\t%s\n" r k

  where

    sortFreq = sortBy
        (\ (k :: String, v :: Double) (k', v') ->
            (compare v' v) `mappend` (compare k k'))

    printFreq :: [(String, Double)] -> IO ()
    printFreq l = forM_ (sortFreq l) $ uncurry (printf "%s %.3f\n")
