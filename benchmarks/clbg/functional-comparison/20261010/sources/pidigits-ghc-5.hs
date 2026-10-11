{- The Computer Language Benchmarks Game
 - https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 -
 - contributed by W. Gordon Goodsman
 - translated from the C version ontributed by Michael Ganss
 - derived from PHP version that was
 - contributed by Oleksii Prudkyi
 - port from pidigits.lua-5.lua (Mike Pall, Wim Couwenberg)
 - modified by Craig Russell
 - 
 - Original C version by Mr Ledrug
 -
-- compile with "ghc -threaded PiDigitsN"
-}

-- {-# LANGUAGE UnboxedTuples, UnliftedFFITypes #-}
-- {-# OPTIONS_GHC -O2 -fllvm -rtsopts #-}

import System.Environment ( getArgs )
import Data.Int ( Int32 )
import Data.Word ( Word32 )
import Control.Monad ( when )
import GHC.Exts ( Ptr )
import Foreign.Ptr ( IntPtr, castPtr )
import Foreign.Storable ( Storable(..) )
import Foreign.Marshal.Alloc ( alloca )

-- Pseudo FFI CTypes
type CInt = Int32
type CWord = Word32

data MPZ = MPZ { alloc :: !Int32, size :: !Int32, ptr :: !IntPtr } deriving Show

-- to make foreign pointers work...
instance Storable MPZ where
  sizeOf mpz = sizeOf (0:: Int32) * 2 + alignment mpz
  alignment _ = sizeOf (0 :: IntPtr)
  peek mpzp = do
    a <- peekElemOff (castPtr mpzp) 0
    s <- peekElemOff (castPtr mpzp) 1
    p <- peekByteOff mpzp (sizeOf (0 :: Int32) * 2)
    return $! MPZ a s p
  poke ptrmpz (MPZ a s p) = do
    _ <- pokeElemOff (castPtr ptrmpz) 0 a
    _ <- pokeElemOff (castPtr ptrmpz) 1 s
    _ <- pokeByteOff ptrmpz (sizeOf (0 :: Int32) * 2) p
    return ()

foreign import ccall unsafe "gmp.h __gmpz_cmp"
  mpzCmp :: Ptr MPZ -> Ptr MPZ -> IO CInt

foreign import ccall unsafe "gmp.h __gmpz_init_set_ui"
  mpzInitSetUi :: Ptr MPZ -> CWord -> IO ()

foreign import ccall unsafe "gmp.h __gmpz_get_ui"
  mpzGetUi :: Ptr MPZ -> IO CWord

foreign import ccall unsafe "gmp.h __gmpz_add"
  mpzAdd :: Ptr MPZ -> Ptr MPZ -> Ptr MPZ -> IO ()

foreign import ccall unsafe "gmp.h __gmpz_mul"
  mpzMul :: Ptr MPZ -> Ptr MPZ -> Ptr MPZ -> IO ()

foreign import ccall unsafe "__gmpz_mul_si"
  mpzMulSi :: Ptr MPZ -> Ptr MPZ -> CInt -> IO ()

foreign import ccall unsafe "gmp.h __gmpz_tdiv_q"
  mpzTdivQ :: Ptr MPZ -> Ptr MPZ -> Ptr MPZ -> IO ()

main :: IO ()
main = doit . read . head =<< getArgs

doit :: Int -> IO ()
doit numdgs =
    alloca $ \ n1 -> alloca $ \ n2 -> alloca $ \ d ->
      alloca $ \ u -> alloca $ \ v -> alloca $ \ w -> do
        mpzInitSetUi n1 4; mpzInitSetUi n2 3; mpzInitSetUi d 1
        mpzInitSetUi u 0; mpzInitSetUi v 0; mpzInitSetUi w 0
        loop 0 9 0 n1 n2 d u v w where
  loop i n k n1 n2 d u v w = do
    if i >= numdgs then
        when (n < 9) $ -- clean up printing final if necessary!
          putStrLn $ replicate n ' ' ++ "\t:" ++ show numdgs else do
      mpzTdivQ u n1 d; mpzTdivQ v n2 d
      cmp <- mpzCmp u v
      if cmp /= 0 then do
        -- produce next term...
        let k2 = 2 * k + 1
        mpzMulSi u n1 k2; mpzAdd v n2 n2; mpzMulSi w n1 k; mpzAdd n1 u v
        mpzMulSi u n2 (k + 3); mpzAdd n2 w u; mpzMulSi d d (k2 + 2)
        loop i n (k + 1) n1 n2 d u v w
      else do let ni = i + 1
              c <- mpzGetUi u 
              putStr $ show c; when (n <= 0) $ putStrLn $ "\t:" ++ show ni
              -- eliminate just-printed digit...
              mpzMulSi u u (-10); mpzMul u d u; mpzMulSi n1 n1 10
              mpzAdd n1 n1 u; mpzMulSi n2 n2 10; mpzAdd n2 n2 u
              loop ni (if n <= 0 then 9 else n - 1) k n1 n2 d u v w
