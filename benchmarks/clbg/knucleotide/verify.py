#!/usr/bin/env python3
"""Independent collection conformance: all keys, boundary lengths and alphabets."""
import argparse,collections,json,os,pathlib,random,subprocess,tempfile
parser=argparse.ArgumentParser();parser.add_argument("--runtime",action="store_true");parser.add_argument("--simd",action="store_true");args=parser.parse_args()
configurations=[(1,"source","0"),(256,"source","0"),(4096,"source","0"),(1,"dag","1"),(7,"dag","1"),(8,"dag","1"),(255,"dag","1"),(256,"dag","1"),(4096,"dag","1")]
if args.runtime:configurations=[(1,"source","0"),(256,"dag","1"),(4096,"dag","1")]
if args.simd:configurations=[(1,"source","0"),(8,"dag","1"),(255,"dag","1"),(4096,"dag","1")]
policies=[(1,False)] if not args.runtime else [(1,False),(1,True),(2,True),(4,True),(8,False),(8,True)]
policies=[(p,n,s) for p,n in policies for s in ([False,True] if args.simd else [False])]
HERE=pathlib.Path(__file__).resolve().parent;ROOT=HERE.parents[2]
TARGET=ROOT/'benchmarks/clbg/nbody/mech-driver/target'
random.seed(20261007)
source=['frequencies(h<{u64:u64}>, bits<u64>, width<u64>, alphabet<string>) = (output<string>) :=\n  keys := sequence/decode(map/keys(h), bits, width, alphabet)\n  counts := map/values(h)\n  order := sequence/sort-indices(counts, "desc", keys, "asc")\n  percentages := 100.0 * number/to-f64(sequence/select(counts, order)) / number/to-f64(stats/sum(counts))\n  output := string/concat(text/join-terminated(text/zip(sequence/select(keys, order), number/format(percentages, 3), " "), "\\n"), "\\n").\nlookup(counts<{u64:u64}>, query<string>, bits<u64>, alphabet<string>) = (output<string>) :=\n  key := sequence/pack(bytes/encode(query, alphabet, "exact"), bits)\n  output := string/concat(number/format(map/get(counts, key), 0), string/concat("\\t", string/concat(query, "\\n"))).\n', 'input := io/read()'];data=bytearray();expected=bytearray();checks=[]
for case,(alphabet,bits,n) in enumerate([('AB',1,n) for n in (0,1,2,3,17,18,19,63,64,65,255,256,257,4095,4096,4097,8201)]+[('ABCDEFGH',3,517)]+[('ACGT',2,n) for n in (0,1,2,3,7,8,9,15,16,17,18,19,31,32,33,255,256,257,1024,4095,4096,4097,8201)]):
 seq=''.join(random.choice(alphabet) for _ in range(n));prefix=f'>CASE{case} '
 data.extend((prefix+'test\r\n').encode())
 for i in range(0,n,37):data.extend((seq[i:i+37].lower()+'\r\n').encode())
 source.append(f's{case} := bytes/encode(text/record(input, "{prefix}", ">"), "{alphabet}", "ignore-ascii-case")')
 widths=sorted({1,2,3,6,12,18,64//bits})
 for width in widths:
  name=f'h{case}-{width}'
  source.append(f'{name} := map/histogram(sequence/packed-windows(s{case}, {bits}, {width}))')
  source.append(f'io/write(frequencies({name}, {bits}, {width}, "{alphabet}"))')
  counts=collections.Counter(seq[i:i+width] for i in range(max(0,n-width+1)));total=sum(counts.values())
  for key,count in sorted(counts.items(),key=lambda x:(-x[1],x[0])):expected.extend(f'{key} {100.0*count/total:.3f}\n'.encode())
  expected.extend(b'\n');query=alphabet[0]*width
  source.append(f'io/write(lookup({name}, "{query}", {bits}, "{alphabet}"))')
  expected.extend(f'{counts[query]}\t{query}\n'.encode());checks.append({'alphabet':alphabet,'length':n,'width':width,'all_keys':len(counts),'updates':total})
# Verify following records never enter the selected sequence and no final newline is required.
data.extend(b'>AFTER final\nTHIS_IS_NOT_IN_ANY_ALPHABET')
with tempfile.TemporaryDirectory(prefix='mech-sequence-conformance-') as temp:
 p=pathlib.Path(temp)/'generic.mec';p.write_text('\n'.join(source)+'\n')
 for batch,schedule,pack8 in configurations:
  env=os.environ|{'CARGO_TARGET_DIR':str(TARGET),'MECH_SEQUENCE_SOURCE':str(p),'MECH_SEQUENCE_BATCH':str(batch),'MECH_SEQUENCE_SCHEDULE':schedule,'MECH_SEQUENCE_PACK8':pack8,'MECH_SEQUENCE_SIMD':'1' if args.simd and pack8=='1' else '0','MECH_SEQUENCE_FUSE_ENCODE':'1' if args.runtime else '0','RAYON_NUM_THREADS':'8','RUSTFLAGS':'-D warnings -C target-cpu=native'}
  subprocess.run(['cargo','build','--release','--locked','--manifest-path',str(HERE/'mech-driver/Cargo.toml')],env=env,cwd=ROOT,check=True)
  for partitions,narrow,standard in policies:
   runtime_env=env|{'MECH_SEQUENCE_PARTITIONS':str(partitions),'MECH_SEQUENCE_NARROW':str(int(narrow)),'MECH_SEQUENCE_STD_MAP':str(int(standard))}
   result=subprocess.run([str(TARGET/'release/mech-clbg-knucleotide')],input=data,env=runtime_env,capture_output=True,check=True)
   if result.stdout!=expected:
    (HERE/'.build/verify-expected.txt').write_bytes(expected);(HERE/'.build/verify-actual.txt').write_bytes(result.stdout)
    raise AssertionError(f'batch {batch}: independent all-key output differs')
   print(f'batch={batch} schedule={schedule} pack8={pack8} partitions={partitions} narrow={narrow} std_map={standard}: {len(checks)} all-key histograms and lookups passed',flush=True)
   # An out-of-alphabet byte must be rejected without native invocation.
   bad=data.replace(b'>CASE0 test\r\n',b'>CASE0 test\r\nZ\r\n',1)
   rejected=subprocess.run([str(TARGET/'release/mech-clbg-knucleotide')],input=bad,env=runtime_env,capture_output=True)
   assert rejected.returncode==1 and b'not in the alphabet' in rejected.stderr
# Test the single-consumer IO fusion, including its EOF consumption effect.
with tempfile.TemporaryDirectory(prefix='mech-record-fusion-') as temp:
 p=pathlib.Path(temp)/'record.mec'
 p.write_text('frequencies(h<{u64:u64}>, bits<u64>, width<u64>, alphabet<string>) = (output<string>) :=\n  keys := sequence/decode(map/keys(h), bits, width, alphabet)\n  counts := map/values(h)\n  order := sequence/sort-indices(counts, "desc", keys, "asc")\n  percentages := 100.0 * number/to-f64(sequence/select(counts, order)) / number/to-f64(stats/sum(counts))\n  output := string/concat(text/join-terminated(text/zip(sequence/select(keys, order), number/format(percentages, 3), " "), "\\n"), "\\n").\nlookup(counts<{u64:u64}>, query<string>, bits<u64>, alphabet<string>) = (output<string>) :=\n  key := sequence/pack(bytes/encode(query, alphabet, "exact"), bits)\n  output := string/concat(number/format(map/get(counts, key), 0), string/concat("\\t", string/concat(query, "\\n"))).\n'+'s := bytes/encode(text/record(io/read(), ">FRUIT ", ">"), "AB", "ignore-ascii-case")\nh := map/histogram(sequence/packed-windows(s, 1, 2))\nio/write(frequencies(h, 1, 2, "AB"))\nio/write(io/read())\n')
 env=os.environ|{'CARGO_TARGET_DIR':str(TARGET),'MECH_SEQUENCE_SOURCE':str(p),'MECH_SEQUENCE_BATCH':'256','MECH_SEQUENCE_SCHEDULE':'dag','MECH_SEQUENCE_PACK8':'1','MECH_SEQUENCE_SIMD':'1' if args.simd else '0','MECH_SEQUENCE_STD_MAP':'1' if args.simd else '0','MECH_SEQUENCE_FUSE_ENCODE':'1' if args.runtime else '0','MECH_SEQUENCE_PARTITIONS':'8','MECH_SEQUENCE_NARROW':'1','RAYON_NUM_THREADS':'8','RUSTFLAGS':'-D warnings -C target-cpu=native'}
 subprocess.run(['cargo','build','--release','--locked','--manifest-path',str(HERE/'mech-driver/Cargo.toml')],env=env,cwd=ROOT,check=True)
 output=subprocess.run([str(TARGET/'release/mech-clbg-knucleotide')],env=env,input=b'>OTHER ignore\nIGNORE\n>FRUIT test\r\nabba\r\n>AFTER discard\nINVALID',capture_output=True,check=True).stdout
 assert output==b'AB 33.333\nBA 33.333\nBB 33.333\n\n'
 print('fused record selection and subsequent EOF read passed',flush=True)
(HERE/'results').mkdir(exist_ok=True)
(HERE/('results/revised-simd-conformance.json' if args.simd else 'results/revised-runtime-conformance.json' if args.runtime else 'results/revised-generic-conformance.json')).write_text(json.dumps({'configurations': [{'batch':b,'schedule':s,'pack8':p=='1'} for b,s,p in configurations], 'runtime_policies':[{'partitions':p,'narrow':n,'standard_map':s} for p,n,s in policies], 'fused_encoding':args.runtime,'simd':args.simd,'checked_histograms':checks,'exact_all_key_frequency_and_lookup_output':True,'invalid_encoding_rejected':True,'fused_record_and_eof_effect':True,'input_cases':'CRLF, lowercase encoding, following records, absent keys, empty/short input, batch tails, packed 64-bit keys, non-DNA alphabet'},indent=2)+'\n')
