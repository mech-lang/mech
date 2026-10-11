-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Inspired by version contributed by Branimir Maksimovic
-- Updated by Callan McGill:
--   - to use hashmap from unordered containers
--   - bit encoding of 'a', 'c', 'g', 't'
--   - Vector streaming framework to re-write algorithm
-- Fix mixup of order in 2-sequences by Artem Pelenitsyn

module Main where

import qualified Control.Concurrent.ParallelIO.Global as ParallelIO
import           Data.Bits
import           Data.ByteString                      (ByteString)
import qualified Data.ByteString                      as ByteString
import qualified Data.ByteString.Char8                as Char8
import           Data.ByteString.Internal             (toForeignPtr)
import           Data.Coerce
import           Data.Hashable
import qualified Data.HashMap.Internal                as Internal
import           Data.HashMap.Strict                  (HashMap)
import qualified Data.HashMap.Strict                  as HashMap
import           Data.List                            (foldl')
import           Data.Primitive.PVar
import qualified Data.Vector                          as Vector
import qualified Data.Vector.Algorithms.Intro         as Sort
import           Data.Vector.Fusion.Stream.Monadic    (Step (..), Stream (..))
import qualified Data.Vector.Fusion.Stream.Monadic    as Stream
import qualified Data.Vector.Storable                 as Storable
import           Foreign.Ptr                          (castPtr, plusPtr)
import           GHC.Word
import qualified Text.Builder                         as Builder


main :: IO ()
main = do
  let
    skip = do
      l <- ByteString.getLine
      if Char8.isPrefixOf ">THREE" l
          then return ()
          else skip
  skip
  s <- ByteString.getContents
  let sv = byteStringToVector s
  let
    content =
     -- Remove newlines and only keep the second and third
     -- bit of each character as in many of the faster solutions.
        Storable.map (\b -> (b .&. 0b110) `shiftR` 1)
      . Storable.filter (/= 10) $ sv
  -- return results in parallel.
  results <- ParallelIO.parallel (actions content)
  -- Put builder results to stdout.
  Builder.putToStdOut (foldl' (<>) mempty results)

{-# inline actions #-}
-- Tasks to be run in parallel.
actions :: Storable.Vector Word8 -> [IO Builder.Builder]
actions content =
  [ writeFrequencies1 content
  , writeFrequencies2 content
  , writeCount content "GGT"
  , writeCount content "GGTA"
  , writeCount content "GGTATT"
  , writeCount content "GGTATTTTAATT"
  , writeCount content "GGTATTTTAATTTATAGT"
  ]

-- Write the one-letter frequences into text builder.
writeFrequencies1 :: Storable.Vector Word8 -> IO Builder.Builder
writeFrequencies1 input = do
    hm <- calculate1B input
    mv <- Vector.unsafeThaw . Vector.fromList . HashMap.toList $ hm
    Sort.sortBy (\(_,x) (_,y) -> y `compare` x) mv
    sorted :: Vector.Vector (Word8, Int)  <- Vector.unsafeFreeze $ mv
    let
      inputLength :: Double
      inputLength = fromIntegral (Storable.length input)
    let b = Vector.foldl' (\acc (k,v)->
                      let
                        perc :: Double
                        perc = 100 * (fromIntegral v)/inputLength
                      in
                          acc
                       <> Builder.char (decode $ k)
                       <> Builder.char ' '
                       <> Builder.fixedDouble 3 perc
                       <> Builder.char '\n') mempty sorted
    pure (b <> Builder.char '\n')

-- Write the two-letter frequencies into text builder.
writeFrequencies2 :: Storable.Vector Word8 -> IO Builder.Builder
writeFrequencies2 input = do
    hm <- calculate2B input
    mv <- Vector.unsafeThaw . Vector.fromList . HashMap.toList $ hm
    Sort.sortBy (\(_,x) (_,y) -> y `compare` x) mv
    sorted :: Vector.Vector (Word16, Int)  <- Vector.unsafeFreeze $ mv
    let
      inputLength :: Double
      inputLength  = fromIntegral (Storable.length input) - 1
    let b = foldl' (\acc (k,v)->
                      let
                        perc :: Double
                        perc = 100 * (fromIntegral v)/inputLength
                      in
                          acc
                       <> decode16 k
                       <> Builder.char ' '
                       <> Builder.fixedDouble 3 perc
                       <> Builder.char '\n') mempty  sorted
    pure (b <> Builder.char '\n')

-- Convert byte back to letter.
decode :: Word8 -> Char
decode 0 = 'A'
decode 1 = 'C'
decode 2 = 'T'
decode 3 = 'G'
decode _ = error "decode: encountered unexpected byte"

-- Convert pair of bytes back to pair of letters
decode16 :: Word16 -> Builder.Builder
decode16 0b0000000000000000 = Builder.text "AA"
decode16 0b0000000000000001 = Builder.text "CA"
decode16 0b0000000000000010 = Builder.text "TA"
decode16 0b0000000000000011 = Builder.text "GA"
decode16 0b0000000100000000 = Builder.text "AC"
decode16 0b0000000100000001 = Builder.text "CC"
decode16 0b0000000100000010 = Builder.text "TC"
decode16 0b0000000100000011 = Builder.text "GC"
decode16 0b0000001000000000 = Builder.text "AT"
decode16 0b0000001000000001 = Builder.text "CT"
decode16 0b0000001000000010 = Builder.text "TT"
decode16 0b0000001000000011 = Builder.text "GT"
decode16 0b0000001100000000 = Builder.text "AG"
decode16 0b0000001100000001 = Builder.text "CG"
decode16 0b0000001100000010 = Builder.text "TG"
decode16 0b0000001100000011 = Builder.text "GG"
decode16 n                  = error $ "decode16: unexpected bits: " <> show n


-- Compute hashmap of single character occurences
calculate1B :: Storable.Vector Word8 -> IO (HashMap Word8 Int)
calculate1B input = Storable.foldM' updateMap HashMap.empty input >>= traverse readPVar
  where
    updateMap
      :: HashMap Word8 (PVar Int RW)
      -> Word8
      -> IO (HashMap Word8 (PVar Int RW))
    updateMap freqmap word =
         case HashMap.lookup word freqmap of
            Nothing ->
              do
                ref <- newPVar 1
                let
                  freqmap'
                  -- Use insertNewKey as we know key is not present
                      = Internal.insertNewKey (fromIntegral word) word ref freqmap
                pure freqmap'
                   -- Mutate reference over copying hashmap on insert.
            Just x -> modifyPVar_ x (+1) >> pure freqmap


-- Compute hashmap of two-character occurences.
calculate2B :: Storable.Vector Word8 -> IO (HashMap Word16 Int)
calculate2B input = fold16M updateMap HashMap.empty input >>= traverse readPVar
  where
    updateMap
      :: HashMap Word16 (PVar Int RW)
      -> Word16
      -> IO (HashMap Word16 (PVar Int RW))
    updateMap freqmap word =
         case HashMap.lookup word freqmap of
            Nothing ->
              do
                ref <- newPVar 1
                let
                  freqmap'
                  -- Use insertNewKey as we know key is not present
                      = Internal.insertNewKey (fromIntegral word) word ref freqmap
                pure freqmap'
                   -- Mutate reference over copying hashmap on insert.
            Just x -> modifyPVar_ x (+1) >> pure freqmap


{-# INLINE writeCount #-}
-- Write number of occurences of string in input.
writeCount :: Storable.Vector Word8 -> ByteString -> IO Builder.Builder
writeCount input string = do
    let size = Char8.length string
    let stringV = byteStringToVector string
    let encoded = Storable.map (\b -> (b .&. 0b110) `shiftR` 1) $ stringV
    hm <- tcalculate input size
    let
      v :: Int
      v = maybe 0 id $ HashMap.lookup (coerce encoded) hm
    let b = Builder.unsignedDecimal v <> Builder.char '\t' <> Builder.asciiByteString string
    pure (b <> Builder.char '\n')

-- Compute hashmaps in parallel over given increment
-- and then re-combine.
tcalculate :: Storable.Vector Word8 -> Int -> IO (HashMap Incremental Int)
tcalculate input size = do
    let
      computeHashMaps = map (\i -> calculate input i size 64) [0..63]
    results <- ParallelIO.parallel computeHashMaps
    return
      $ foldl' (\ acc hm -> HashMap.unionWith (+) acc hm) HashMap.empty results

-- Compute hashmap from given beginning point with the given increment.
calculate :: Storable.Vector Word8 -> Int -> Int -> Int -> IO (HashMap Incremental Int)
calculate input !beg !size !incr =
    Stream.foldM' updateHashMap HashMap.empty (fromVectorWithInc beg size incr end input) >>=
    traverse readPVar
  where
    end = Storable.length input + 1 - size
    updateHashMap
      :: HashMap Incremental (PVar Int RW)
      -> Incremental
      -> IO (HashMap Incremental (PVar Int RW))
    updateHashMap freqmap slice =
         case HashMap.lookup slice freqmap of
            Nothing -> do
              ref <- newPVar 1
              let
                freqmap'
                    = Internal.insertNewKey (fromIntegral (hash slice)) slice ref freqmap
              pure freqmap'
            Just x -> modifyPVar_ x (+1) >> pure freqmap


newtype Incremental = Incremental (Storable.Vector Word8)
  deriving newtype Eq

-- Use a custom hashable instance using the bit values we
-- have extracted from the input
instance Hashable Incremental where
  hash (Incremental v) =
    Storable.foldl'
    (\ acc w -> (acc `shiftL` 2) .|. (fromIntegral w))
    0
    v
  hashWithSalt _ = error "hashWithSalt not implemented."

-- Convert a bytestring to a storable vector without copying.
-- Taken from: bytestring-to-vector package
byteStringToVector :: ByteString -> Storable.Vector Word8
byteStringToVector bs = vec where
    vec = Storable.unsafeFromForeignPtr fptr off len
    (fptr, off, len) = toForeignPtr bs


fold16M :: (a -> Word16 -> IO a) -> a -> Storable.Vector Word8 -> IO a
fold16M f v vec = do
     Storable.unsafeWith vec $ \ptr -> go v ptr (ptr `plusPtr` (len - 1))
  where
    len = Storable.length vec
        -- tail recursive; traverses array left to right
    go !z !p end | p == end = return z
                  | otherwise =
                    do
                      x <- peek (castPtr @_ @Word16 p)
                      z' <- f z x
                      go z' (p `plusPtr` 1) end



{-# inline fromVectorWithInc #-}
fromVectorWithInc
  :: forall m . (Monad m)
  => Int
  -> Int
  -> Int
  -> Int
  -> Storable.Vector Word8
  -> Stream m Incremental
fromVectorWithInc beg size inc end v = Stream step beg
  where
    {-# INLINE step #-}
    step :: Int -> m (Step Int Incremental)
    step !i | i >= end = return Done
            | otherwise = return $ Yield (coerce (Storable.unsafeSlice i size v)) (i+inc)
