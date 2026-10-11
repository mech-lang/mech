-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Branimir Maksimovic

import qualified Data.ByteString.Char8 as S
import qualified Data.HashTable.IO as H
import qualified Data.Map as M
import Text.Printf
import Data.List
import Data.Maybe
import Data.Char
import Data.IORef
import Control.Concurrent

main = do
    let skip = do
            l <- S.getLine
            if S.isPrefixOf (S.pack ">THREE") l
                then return ()
                else skip
    skip
    s <- S.getContents
    let content = S.map toUpper $ S.filter ((/=) '\n') s;
    mapM_ (execute content) actions
    
data Actions = I Int | S String
actions = [I 1,I 2,
           S "GGT",S "GGTA",S "GGTATT",S "GGTATTTTAATT",S "GGTATTTTAATTTATAGT"]
execute content (I i) = writeFrequencies content i
execute content (S s) = writeCount content s

writeFrequencies :: S.ByteString -> Int -> IO ()
writeFrequencies input size = do
    mp <- tcalculate input size
    let sorted = sortBy (\(_,x) (_,y) -> y `compare` x) $ M.toList mp
        sum = fromIntegral ((S.length input) + 1 - size)
    mapM_ (\(k,v)-> do
        printf "%s %.3f\n" 
            (S.unpack k) ((100 * (fromIntegral v)/sum)::Double)) sorted
    putChar '\n'

writeCount :: S.ByteString -> String -> IO ()
writeCount input string = do
    let size = length string
    mp <- tcalculate input size
    let v = fromJust $ M.lookup (S.pack string) mp
    printf "%d\t%s\n" v string

tcalculate :: S.ByteString -> Int -> IO (M.Map S.ByteString Int) 
tcalculate input size = do
    let 
        l = [0..63]
        actions = map (\i -> calculate input i size (length l)) l
    vars <- mapM (\action -> do
                    var <- newEmptyMVar
                    forkIO $ do
                        answer <- action
                        putMVar var answer
                    return var) actions
    let result = M.empty
    results <- mapM takeMVar vars
    return $ foldl (\res m -> foldl 
                               (\m (k,v)->M.insertWith (+) k v m)
                                res m)
                   result results

calculate :: S.ByteString -> Int -> Int -> Int -> IO [(S.ByteString,Int)]
calculate input beg size incr = do
    let updateMap freqmap word = do
           lu <- H.lookup freqmap word
           case lu of 
            Nothing -> do
                ref <- newIORef 1
                H.insert freqmap word ref
            Just x -> modifyIORef' x (+1)
           return freqmap
        word inp pos sz = S.take size $ S.drop pos inp
        calculate' freqmap i
            | i >= ((S.length input)+1 - size) = return ()
            | otherwise = do
                ht <- updateMap freqmap $ word input i size
                calculate' ht (i+incr)
    freqmap <- H.new :: IO (HashTable S.ByteString (IORef Int))
    calculate' freqmap beg
    lst <- H.toList freqmap
    mapM (\(x,y)-> do
            v <- readIORef y
            return (x,v)) lst
    

type HashTable k v = H.BasicHashTable k v
