"""Keep originals intact; pin separately labelled portability/work corrections."""
import hashlib,json,re
from pathlib import Path
from fetch import metrics
D=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def main():
    manifest=json.loads((D/'manifest.json').read_text());rows=[]
    for row in manifest['programs']:
        text=Path(row['source']).read_text();before=text;notes=[];name=row['id']
        if row['benchmark']=='binarytrees':
            if row['language']=='haskell':
                text=text.replace('data Tree = Nil | Node Tree Tree','data Tree = Nil | Node !Tree !Tree')
                text=re.sub(r'Node i Nil Nil',r'Node i (leafNil i) (leafNil i)',text)
                text=re.sub(r'Node Nil Nil',r'Node (leafNil 0) (leafNil 0)',text)
                defs=re.findall(r'^(make(?:Par[24]|\')?)\s+(?!::)',text,re.M)
                barriers='\n'.join('{-# NOINLINE '+n+' #-}' for n in sorted(set(defs)))
                text+='\n-- Preserve per-node allocation, including depth-zero leaves.\n'+barriers+'\n{-# OPAQUE leafNil #-}\nleafNil :: Int -> Tree\nleafNil _ = Nil\n'
                if name=='binarytrees-ghc-2':
                    text=text.replace('    let longT = make n\n        longC = check longT\n    longC `seq` pure ()','    let !longT = make n')
                    text=text.replace('makeOutput "long lived tree" n longC','makeOutput "long lived tree" n (check longT)')
                notes+=['Keep strict child fields and separately allocate every leaf using an opaque sentinel producer. Prevent constructor/traversal fusion with NOINLINE constructor annotations; disable CSE and full-laziness for this allocation benchmark only. All annotations/barriers count toward LOC.']
                if name=='binarytrees-ghc-2':notes+=['Move the long-lived traversal after all temporary trees and force its complete strict construction before temporary work.']
            else:
                text=text.replace('Node(Empty, Empty)','Node(Sys.opaque_identity Empty, Empty)')
                notes+=['Use standard Sys.opaque_identity to prevent the compiler from lifting a shared constant leaf. Every interior and depth-zero node allocates the same two child slots.']
        elif row['benchmark']=='fasta':
            if name=='fasta-ghc-2':
                text=text.replace('Float','Double');notes+=['Use binary64 sampling and cumulative probabilities for the common cohort workload.']
            elif name in ('fasta-ghc-4','fasta-ghc-5'):
                text=text.replace('Float','Double')
                text=text.replace('-> V.Vector Word32\n  -> V.Vector Word8','-> V.Vector Double\n  -> V.Vector Word8')
                text=text.replace(':: V.Vector Word32\n  -> V.Vector Word8',':: V.Vector Double\n  -> V.Vector Word8')
                # Original PRNG buffers retain Word32 values; only probability tables change.
                text=text.replace('V.Vector (Word32, Word8)','V.Vector (Double, Word8)')
                text=text.replace('floor (fromIntegral im * p\')','p\'')
                text=text.replace('(V.findIndex (c <=) p0)','(V.findIndex (fromIntegral c / fromIntegral im <) p0)')
                notes+=['Replace integer-scaled lookup thresholds with cumulative binary64 probabilities; divide every newly generated PRNG value by IM before the original search. Retain the original threading and all LCG steps.']
            elif name=='fasta-ghc-6':
                text=text.replace('type Dist = A.Array A.P Int Cumul','type Dist = A.Array A.P Int Double')
                text=text.replace('[(Char, Float)]','[(Char, Double)]')
                text=text.replace('init (map (floor . (* fromIntegral im)) (scanl1 (+) bs)) ++ [im]','scanl1 (+) bs')
                text=text.replace('c <= U.unsafeIndex p1 i','fromIntegral c / fromIntegral im < U.unsafeIndex p1 i')
                notes+=['Replace integer-scaled lookup thresholds with cumulative binary64 probabilities; explicitly divide each fresh PRNG value before comparison. Retain sequential RNG generation and ordered buffered output.']
            elif name=='fasta-ocaml-6':
                text=text.replace('let im_inv_f = 1. /. float im','let im_f = float im').replace('float n *. im_inv_f','float n /. im_f')
                notes+=['Use explicit binary64 division per RNG value for the common cohort workload, retaining cumulative binary search.']
        elif name in ('knucleotide-ocaml-2','knucleotide-ocaml-3'):
            text=text.replace('Bytes.length dna - k - 1','Bytes.length dna - k').replace('String.length dna - k - 1','String.length dna - k')
            text=text.replace('let c = 0x40000-1','let c = 16').replace('let c = 0x40000','let c = 16')
            notes+=['Include the final valid overlapping window for every one of the seven width-specific hash maps.']
            notes+=['Grow each standard-library hash table from a small initial capacity (16) instead of preallocating 262,144 slots.']
        elif name=='knucleotide-ocaml-1':
            text=text.replace('let size = 0x40000','let size = 16')
            notes+=['Grow each standard-library hash table from a small initial capacity (16), as the current benchmark description requires. All seven overlapping count maps are retained.']
        elif name=='knucleotide-ghc-1':
            text=text.replace('fromJust $ M.lookup','fromMaybe 0 $ M.lookup')
            notes+=['Print zero for an absent requested key instead of failing at small input. All seven complete count maps are still built.']
        elif name=='knucleotide-ghc-3':
            text=text.replace('import qualified Text.Builder                         as Builder','import qualified Data.ByteString.Builder              as Builder\nimport Text.Printf (printf)\nimport System.IO (stdout)')
            text=text.replace('Builder.putToStdOut','Builder.hPutBuilder stdout').replace('Builder.char','Builder.char8').replace('Builder.text','Builder.string8')
            text=text.replace('Builder.fixedDouble 3 perc','Builder.string8 (printf "%.3f" perc)').replace('Builder.unsignedDecimal','Builder.intDec').replace('Builder.asciiByteString','Builder.byteString')
            text=text.replace('Internal.insertNewKey (fromIntegral word) word ref','Internal.insertNewKey (fromIntegral (hash word)) word ref')
            first='Sort.sortBy (\\(_,x) (_,y) -> y `compare` x) mv'
            text=text.replace(first,'Sort.sortBy (\\(k,x) (k\',y) -> (y `compare` x) <> compare (decode k) (decode k\')) mv',1)
            text=text.replace(first,'Sort.sortBy (\\(k,x) (k\',y) -> (y `compare` x) <> compare (Builder.toLazyByteString (decode16 k)) (Builder.toLazyByteString (decode16 k\'))) mv',1)
            notes+=['Use the standard bytestring Builder and printf APIs in place of the retired Text.Builder API. Derive internal one-/two-byte insertion hashes using the current Hashable instance, matching subsequent lookups. Every seven-width map update remains in Haskell.']
            notes+=['Break equal-frequency ties by the decoded nucleotide key, as required.']
        elif name=='mandelbrot-ghc-2':
            text=text.replace('coords = [T 1 0','coords = [T 0 0')
            notes+=['Start the byte counter at zero so every output byte and its eight pixels are computed.']
        elif name=='mandelbrot-ghc-3':
            text=text.replace('writeWord8Array# pixels (rowstart +# (x `uncheckedIShiftRA#` 6#)) (wordToWord8# (mand64 r0 (x `uncheckedIShiftRA#` 1#) init_i))','writeWordArray# pixels (rowstart +# (x `uncheckedIShiftRA#` 6#)) (mand64 r0 (x `uncheckedIShiftRA#` 1#) init_i)')
            notes+=['Write all 64 bits of each eight-byte packed block using GHC writeWordArray#, rather than truncating the block through writeWord8Array#. Retain the published DoubleX2 SIMD recurrence.']
        elif name in ('fannkuchredux-ocaml-3','fannkuchredux-ocaml-4'):
            text=text.replace('let f = ref p.(0) and res = ref 1 in','let f = ref p.(0) and res = ref (if p.(0) = 0 then 0 else 1) in')
            if name.endswith('-3'):text=text.replace('      Perm.next p','      if i < red_hi then Perm.next p')
            notes+=['Count zero flips when the leading value is already in place. Stop advancing permutations after the final assigned rank, avoiding an out-of-bounds terminal rotation. The complete rank interval, checksum and maximum remain in OCaml.']
        if text!=before:
            new=row.copy();identifier=name+'-faithful';source=D/'sources'/(identifier+Path(row['source']).suffix)
            source.write_text(text)
            new.update(id=identifier,key=row['benchmark']+':'+identifier,source=str(source),source_sha256=sha(source),
                source_metrics=metrics(text,row['language']),adaptations=notes,upstream_id=name,local_adaptation=True)
            rows.append(new)
    report=dict(status='corrections-pinned',manifest_sha256=sha(D/'manifest.json'),programs=rows)
    (D/'corrections.json').write_text(json.dumps(report,indent=2)+'\n')
    for row in rows:print(row['id'],row['source_metrics']['nonblank_noncomment_lines'],'LOC',flush=True)
if __name__=='__main__':main()
