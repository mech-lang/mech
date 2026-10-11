-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
--
-- Translated from C #6 to Haskell by Jaro Reinders
--
-- Contributed by Kevin Miller
--
-- ver 2: added a couple of optimizations
-- - Reduced number of times a vector of 8 was checked to see if
--    they had all escaped, similar to Dominic Letz's C #5 entry.
-- - Processed 64 pixels at a time if width was a multiple of 64,
--    thereby reducing writes to the bitmap.

-- GHC 9.2, 9.10 fixes contributed by Artem Pelenitsyn

import GHC.IO
import GHC.Exts
import System.Environment
import qualified Data.ByteString.Char8 as B
import qualified Data.ByteString.Unsafe as B
import Control.Monad

{-# INLINE vecNLE #-}
vecNLE :: DoubleX24# -> Double# -> Int#
vecNLE (# v0, v1, v2, v3 #) f
  =  ((v00 >## f)
  `andI#` (v01 >## f)
  `andI#` (v10 >## f)
  `andI#` (v11 >## f)
  `andI#` (v20 >## f)
  `andI#` (v21 >## f)
  `andI#` (v30 >## f)
  `andI#` (v31 >## f))
  where
    !(# v00, v01 #) = unpackDoubleX2# v0
    !(# v10, v11 #) = unpackDoubleX2# v1
    !(# v20, v21 #) = unpackDoubleX2# v2
    !(# v30, v31 #) = unpackDoubleX2# v3

{-# INLINE pixelsGT #-}
pixelsGT :: DoubleX24# -> Double# -> Word#
pixelsGT (# v0, v1, v2, v3 #) f
  =     ((int2Word# (v00 <=## f)) `uncheckedShiftL#` 7#)
  `or#` ((int2Word# (v01 <=## f)) `uncheckedShiftL#` 6#)
  `or#` ((int2Word# (v10 <=## f)) `uncheckedShiftL#` 5#)
  `or#` ((int2Word# (v11 <=## f)) `uncheckedShiftL#` 4#)
  `or#` ((int2Word# (v20 <=## f)) `uncheckedShiftL#` 3#)
  `or#` ((int2Word# (v21 <=## f)) `uncheckedShiftL#` 2#)
  `or#` ((int2Word# (v30 <=## f)) `uncheckedShiftL#` 1#)
  `or#` ((int2Word# (v31 <=## f)) {- `uncheckedShiftL#` 0# -})
  where
    !(# v00, v01 #) = unpackDoubleX2# v0
    !(# v10, v11 #) = unpackDoubleX2# v1
    !(# v20, v21 #) = unpackDoubleX2# v2
    !(# v30, v31 #) = unpackDoubleX2# v3

{-# INLINE calcSum #-}
calcSum
  :: DoubleX24#
  -> DoubleX2#
  -> (# DoubleX24#, DoubleX24#, DoubleX24# #)
  -> (# DoubleX24#, DoubleX24#, DoubleX24# #)
calcSum
  (# initR0, initR1, initR2, initR3 #)
  initI
  (# (# r00, r10, r20, r30 #)
  ,  (# i00, i10, i20, i30 #)
  ,  (# _, _, _, _ #) #)
  = (# (# r01, r11, r21, r31 #)
    ,  (# i01, i11, i21, i31 #)
    ,  (# sum01, sum11, sum21, sum31 #) #)
  where
    r'0 = r00 `timesDoubleX2#` r00
    i'0 = i00 `timesDoubleX2#` i00
    ri0 = r00 `timesDoubleX2#` i00
    sum01 = r'0 `plusDoubleX2#` i'0
    r01 = (r'0 `minusDoubleX2#` i'0) `plusDoubleX2#` initR0
    i01 = (ri0 `plusDoubleX2#` ri0) `plusDoubleX2#` initI

    r'1 = r10 `timesDoubleX2#` r10
    i'1 = i10 `timesDoubleX2#` i10
    ri1 = r10 `timesDoubleX2#` i10
    sum11 = r'1 `plusDoubleX2#` i'1
    r11 = (r'1 `minusDoubleX2#` i'1) `plusDoubleX2#` initR1
    i11 = (ri1 `plusDoubleX2#` ri1) `plusDoubleX2#` initI

    r'2 = r20 `timesDoubleX2#` r20
    i'2 = i20 `timesDoubleX2#` i20
    ri2 = r20 `timesDoubleX2#` i20
    sum21 = r'2 `plusDoubleX2#` i'2
    r21 = (r'2 `minusDoubleX2#` i'2) `plusDoubleX2#` initR2
    i21 = (ri2 `plusDoubleX2#` ri2) `plusDoubleX2#` initI

    r'3 = r30 `timesDoubleX2#` r30
    i'3 = i30 `timesDoubleX2#` i30
    ri3 = r30 `timesDoubleX2#` i30
    sum31 = r'3 `plusDoubleX2#` i'3
    r31 = (r'3 `minusDoubleX2#` i'3) `plusDoubleX2#` initR3
    i31 = (ri3 `plusDoubleX2#` ri3) `plusDoubleX2#` initI

type DoubleX24# = (# DoubleX2#, DoubleX2#, DoubleX2#, DoubleX2# #)

{-# INLINE mand8 #-}
mand8 :: ByteArray# -> Int# -> DoubleX2# -> Word#
mand8 initRs indexR initI = go1 6# (# initR, i, initSum #) where
  go1 :: Int# -> (# DoubleX24#, DoubleX24#, DoubleX24# #) -> Word#
  go1 0# s =
    let !(# _, _, sum #) = calcSum initR initI (calcSum initR initI s)
    in pixelsGT sum 4.0## -- 0xff##
  go1 n s =
    let s'@(!(# _, _, sum #)) = go2 8# s
    in if isTrue# (vecNLE sum 4.0##) then 0x00## else go1 (n -# 1#) s'

  go2 0# s = s
  go2 n s = go2 (n -# 1#) (calcSum initR initI s)

  i = (# initI, initI, initI, initI #)
  initSum =
    (# broadcastDoubleX2# 0.0##
    ,  broadcastDoubleX2# 0.0##
    ,  broadcastDoubleX2# 0.0##
    ,  broadcastDoubleX2# 0.0##
    #)

  initR0 = indexDoubleX2Array# initRs indexR
  initR1 = indexDoubleX2Array# initRs (indexR +# 1#)
  initR2 = indexDoubleX2Array# initRs (indexR +# 2#)
  initR3 = indexDoubleX2Array# initRs (indexR +# 3#)
  initR = (# initR0, initR1, initR2, initR3 #)

{-# INLINE mand64 #-}
mand64 :: ByteArray# -> Int# -> DoubleX2# -> Word#
mand64 initRs indexR initI = go 8# indexR 0## where
  go 0# _ pix64 = pix64
  go n indexR pix64 = go
    (n -# 1#)
    (indexR +# 4#)
    ((pix64 `uncheckedShiftRL#` 8#) `or#` ((mand8 initRs indexR initI) `uncheckedShiftL#` 56#))

main :: IO ()
main = do
  xs <- getArgs
  let
    widHt = case xs of
      (x : _) -> read x
      _ -> 16000
    widHt# = (\(I# x) -> x) widHt
    pixelLength# = widHt# *# (widHt# `uncheckedIShiftRA#` 3#)

  putStrLn ("P4\n" ++ show widHt ++ " " ++ show widHt)

  pixels <- join $ IO $ \s ->
    case newByteArray# (8# *# widHt#) s of { (# s, r #) ->
    case newByteArray# (8# *# widHt#) s of { (# s, i #) ->
    case 2.0## /## int2Double# widHt# of { a ->
    let
      -- go :: Int# -> State# RealWorld -> (# State# RealWorld, (# ByteArray#, ByteArray# #) #)
      go xy s
        | isTrue# (xy <# widHt#) =
          -- case (\(IO x) -> x) (print (D# (a *## int2Double# xy -## 1.5##))) s of { (# s, () #) ->
          case writeDoubleX2Array# r (xy `uncheckedIShiftRA#` 1#) ((broadcastDoubleX2# a `timesDoubleX2#` packDoubleX2# (# int2Double# xy, int2Double# (xy +# 1#) #)) `minusDoubleX2#` broadcastDoubleX2# 1.5##) s of { s ->
          case writeDoubleArray# i xy (a *## int2Double# xy -## 1.0##) s of { s ->
          case writeDoubleArray# i (xy +# 1#) (a *## int2Double# (xy +# 1#) -## 1.0##) s of { s ->
          go (xy +# 2#) s }}}--}
        | otherwise =
          case unsafeFreezeByteArray# r s of { (# s, r #) ->
          case unsafeFreezeByteArray# i s of { (# s, i #) ->
          (# s, (# r, i #) #) }}
    in case go 0# s of { (# s, (# r0, i0 #) #) ->
    case newAlignedPinnedByteArray# pixelLength# 8# s of { (# s, pixels #) ->
    case newMVar# s of { (# s, var #) ->
    case putMVar# var 0 s of { s ->
    let
      chunkSize# = widHt# `uncheckedIShiftRA#` 6#
      worker s
        | isTrue# (notI# (widHt# `remInt#` 64#)) =
          case newMVar# s of { (# s, thread #) ->
          case fork# (\s ->
            let
              go s = case takeMVar# var s of
                (# s, I# val #)
                  | isTrue# (val <# widHt#) ->
                    let
                      chunk
                        | isTrue# ((widHt# -# val) <=# chunkSize#) = widHt# -# val
                        | otherwise = chunkSize#
                    in
                      -- case (\(IO x) -> x) (putStrLn (show (I# (val +# chunk), widHt))) s of { (# s, () #) ->
                      case putMVar# var (I# (val +# chunk)) s of { s ->
                      let
                        go1 y s
                          | isTrue# (y <# (val +# chunk)) = -- case (\(IO x) -> x) (putStrLn (show (D# (indexDoubleArray# i0 y)))) s of { (# s, () #) ->
                            let
                              init_i = broadcastDoubleX2# (indexDoubleArray# i0 y)
                              rowstart = y *# (widHt# `uncheckedIShiftRA#` 6#)
                              go2 x s
                                | isTrue# (x <# widHt#) =
                                  -- case (\(IO x) y -> x y) (putStrLn (show (D# (indexDoubleArray# r0 x)))) s of { (# s, () #) ->
                                  case writeWord8Array# pixels (rowstart +# (x `uncheckedIShiftRA#` 6#)) (wordToWord8# (mand64 r0 (x `uncheckedIShiftRA#` 1#) init_i)) s of { s ->
                                  go2 (x +# 64#) s } -- }
                                | otherwise = s
                            in go1 (y +# 1#) (go2 0# s) -- }
                          | otherwise = s
                      in go (go1 val s) }
                  | otherwise -> (# putMVar# thread () (putMVar# var (I# val) s), () #)
            in go s) s of
            (# s, _ #) -> (# s, thread #) }
        | otherwise =
          case newMVar# s of { (# s, thread #) ->
          case fork# (\s ->
            let
              go s = case takeMVar# var s of
                (# s, I# val #)
                  | isTrue# (val <# widHt#) ->
                    let
                      chunk
                        | isTrue# ((widHt# -# val) <=# chunkSize#) = widHt# -# val
                        | otherwise = chunkSize#
                    in
                      -- case (\(IO x) -> x) (putStrLn (show (I# (val +# chunk), widHt))) s of { (# s, () #) ->
                      case putMVar# var (I# (val +# chunk)) s of { s ->
                      let
                        go1 y s
                          | isTrue# (y <# (val +# chunk)) = -- case (\(IO x) -> x) (putStrLn (show (D# (indexDoubleArray# i0 y)))) s of { (# s, () #) ->
                            let
                              init_i = broadcastDoubleX2# (indexDoubleArray# i0 y)
                              rowstart = y *# (widHt# `uncheckedIShiftRA#` 3#)
                              go2 x s
                                | isTrue# (x <# widHt#) =
                                  -- case (\(IO x) y -> x y) (putStrLn (show (D# (indexDoubleArray# r0 x)))) s of { (# s, () #) ->
                                  case writeWord8Array# pixels (rowstart +# (x `uncheckedIShiftRA#` 3#)) (wordToWord8# (mand8 r0 (x `uncheckedIShiftRA#` 1#) init_i)) s of { s ->
                                  go2 (x +# 8#) s } -- }
                                | otherwise = s
                            in go1 (y +# 1#) (go2 0# s) -- }
                          | otherwise = s
                      in go (go1 val s) }
                  | otherwise -> (# putMVar# thread () (putMVar# var (I# val) s), () #)
            in go s) s of
            (# s, _ #) -> (# s, thread #) }
    in
      case worker s of { (# s, thread0 #) ->
      case worker s of { (# s, thread1 #) ->
      case worker s of { (# s, thread2 #) ->
      case worker s of { (# s, thread3 #) ->
      case takeMVar# thread0 s of { (# s, () #) ->
      case takeMVar# thread1 s of { (# s, () #) ->
      case takeMVar# thread2 s of { (# s, () #) ->
      case takeMVar# thread3 s of { (# s, () #) ->
      case unsafeFreezeByteArray# pixels s of { (# s, pixels #) ->
      (# s, B.unsafePackAddressLen (I# pixelLength#) (byteArrayContents# pixels) #) }
      }}}}}}}}}}}}}}}
  B.putStr pixels
