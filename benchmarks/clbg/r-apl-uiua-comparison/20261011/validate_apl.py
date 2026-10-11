#!/usr/bin/env python3
"""Canonical and adversarial correctness only; serialized with the full measurement lock."""
from pathlib import Path
import collections, fcntl, hashlib, json, math, os, random, re, subprocess
G=Path(__file__).resolve().parent
C=G.parents[1]
OUT=G/'results/apl'
WORK={'nbody':1000,'spectralnorm':100,'mandelbrot':200,'fannkuchredux':7,'fasta':1000,'knucleotide':1000,'revcomp':1000,'regexredux':30,'binarytrees':10,'pidigits':30}
HASHES={'mandelbrot':(5011,'97610473750700638fc63d13cfa49d339b67c18e7f26b3f9c9acb61e746472d5'),'knucleotide':(242,'baa63c46addb89c63e68ea0055952847fbccf450e89997b2b62234c90e66f500'),'revcomp':(10245,'549dcdcc7df2685145e6cbdfd682469bd9c12ff1beec93c270f8ba447acd4855'),'fasta':(10245,'62d1e8d0df7938d2aefda9a37887e0389231ea72c099c29a51afb6edca1bdc73'),'binarytrees':(223,'b7f92c56b5d8aeb0a4d698842d1d87a57b4909865c3c84e5e10313e16663c3cb')}
report=[]
def digest(data):return hashlib.sha256(data).hexdigest()
def sources(bench):return [(v,G/'sources/apl'/f'{bench}.{v}.apl') for v in ['compact','optimized'] if (G/'sources/apl'/f'{bench}.{v}.apl').exists()]
def execute(bench,variant,n=None,data=None):
 command=[str(G/'run-apl.sh'),'--cc','8' if variant=='optimized' else '1','-f',str(G/'sources/apl'/f'{bench}.{variant}.apl')]+([] if n is None else ['--',str(n)])
 return subprocess.run(command,input=data or b'',stdout=subprocess.PIPE,stderr=subprocess.PIPE,cwd=G,env=os.environ|{'LC_ALL':'C'},timeout=90)
def check(bench,case,expected,n=None,data=None):
 for variant,path in sources(bench):
  p=execute(bench,variant,n,data)
  ok=p.returncode==0 and p.stdout==expected and not re.search(rb'(?:DOMAIN|RANK|LENGTH|INDEX|SYNTAX|VALUE|WS FULL) ERROR',p.stderr)
  row=dict(benchmark=bench,variant=variant,case=case,n=n,input_sha256=digest(data) if data is not None else None,bytes=len(p.stdout),sha256=digest(p.stdout),reference_match=ok,returncode=p.returncode,stderr=p.stderr.decode())
  report.append(row)
  if case=='canonical-small':
   (OUT/f'{bench}.{variant}.small.stdout').write_bytes(p.stdout)
   (OUT/f'{bench}.{variant}.small.stderr').write_bytes(p.stderr)
  print(bench,variant,case,'PASS' if ok else 'FAIL',flush=True)
  if not ok:print('actual',repr(p.stdout[:500]),'expected',repr(expected[:500]),p.stderr.decode()[:1000],flush=True)
def knuc(data):
 reading=False;parts=[]
 for line in data.decode().splitlines():
  if line.startswith('>THREE'):reading=True
  elif reading and line.startswith('>'):break
  elif reading and not line.startswith(';'):parts.append(line.upper())
 seq=''.join(parts);out=[]
 queries={3:'GGT',4:'GGTA',6:'GGTATT',12:'GGTATTTTAATT',18:'GGTATTTTAATTTATAGT'}
 for width in [1,2,3,4,6,12,18]:
  counts=collections.Counter(seq[i:i+width] for i in range(max(0,len(seq)-width+1)))
  if width<=2:
   out.extend(f'{key} {100*count/sum(counts.values()):.3f}\n' for key,count in sorted(counts.items(),key=lambda item:(-item[1],item[0])))
   out.append('\n')
  else:out.append(f'{counts[queries[width]]}\t{queries[width]}\n')
 return ''.join(out).encode()
def revcomp(data):
 mapping=str.maketrans('ACBDGHKMNSRUTWVYacbdghkmnsrutwvy','TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR')
 records=[]
 for line in data.decode().splitlines():
  if line.startswith('>'):records.append([line,[]])
  elif records and not line.startswith(';'):records[-1][1].append(line)
 out=[]
 for header,parts in records:
  seq=''.join(parts).translate(mapping)[::-1]
  out.append(header+'\n');out.extend(seq[i:i+60]+'\n' for i in range(0,len(seq),60))
 return ''.join(out).encode()
def mandelbrot(n):
 out=bytearray(f'P4\n{n} {n}\n'.encode())
 for y in range(n):
  row=[]
  for x in range(n):
   zr=zi=0.0;inside=True;cr=2.0*x/n-1.5;ci=2.0*y/n-1
   for _ in range(50):
    zr,zi=cr+zr*zr-zi*zi,ci+2*zr*zi
    if zr*zr+zi*zi>4:inside=False;break
   row.append(inside)
  row+=[False]*((-n)%8)
  out.extend(sum(int(b)*(1<<(7-j)) for j,b in enumerate(row[i:i+8])) for i in range(0,len(row),8))
 return bytes(out)
def regex(data):
 cleaned=re.sub(b'>[^\n]*\n|\n',b'',data);text=cleaned
 patterns=['agggtaaa|tttaccct','[cgt]gggtaaa|tttaccc[acg]','a[act]ggtaaa|tttacc[agt]t','ag[act]gtaaa|tttac[agt]ct','agg[act]taaa|ttta[agt]cct','aggg[acg]aaa|ttt[cgt]ccct','agggt[cgt]aa|tt[acg]accct','agggta[cgt]a|t[acg]taccct','agggtaa[cgt]|[acg]ttaccct']
 for pattern,replacement in [(b'tHa[Nt]',b'<4>'),(b'aND|caN|Ha[DS]|WaS',b'<3>'),(b'a[NSt]|BY',b'<2>'),(b'<[^>]*>',b'|'),(rb'\|[^|][^|]*\|',b'-')]:text=re.sub(pattern,replacement,text)
 return (''.join(f'{p} {len(re.findall(p.encode(),cleaned))}\n' for p in patterns)+f'\n{len(data)}\n{len(cleaned)}\n{len(text)}\n').encode()
def main():
 OUT.mkdir(parents=True,exist_ok=True)
 for bench,n in WORK.items():
  ip=C/'knucleotide/inputs/knucleotide-input-1000.txt' if bench in ['knucleotide','revcomp'] else C/'regexredux/inputs/regexredux-input-small.txt' if bench=='regexredux' else None
  if bench in HASHES:
   existing=G/'results/apl'/f'{bench}.small.stdout'
   expected=existing.read_bytes();assert (len(expected),digest(expected))==HASHES[bench]
  elif bench=='nbody':expected=b'-0.169075164\n-0.169087605\n'
  elif bench=='spectralnorm':expected=b'1.274219991\n'
  elif bench=='fannkuchredux':expected=b'228\nPfannkuchen(7) = 16\n'
  else:expected=(C/f'{bench}/inputs/{bench}-output-small.txt').read_bytes()
  # Reference text for pidigits is stored with literal \t in this repository.
  if bench=='pidigits':expected=expected.replace(b'\\t',b'\t').replace(b'\\n',b'\n')
  check(bench,'canonical-small',expected,n=None if ip else n,data=ip.read_bytes() if ip else None)
 cases=[(b'>THREE short\nA\n','short-window'),(b'>THREE empty\n>FOUR\nAGG\n','empty-record'),(b'>ONE\nACGT\n','missing-record'),(b'>THREE crlf\r\naCgT\r\n;ignored comment\r\nGGTATT\r\n','comments-crlf'),(b'>THREE ties\nACGTACGTACGT\n>FOUR\nGGGG\n','ties-stop')]
 rng=random.Random(20261011);dna=''.join(rng.choice('ACGT') for _ in range(1003))+'GGTATTTTAATTTATAGT'
 cases.append((('>THREE allwindows\n'+'\n'.join(dna[i:i+37].lower() for i in range(0,len(dna),37))+'\n').encode(),'all-widths-growing-maps'))
 for data,name in cases:check('knucleotide',name,knuc(data),data=data)
 for data,name in [(b'>one\nACBDGHKMNSRUTWVYacbdghkmnsrutwvy\n>empty\n>two\nAACCGGTT\n','all-IUB-empty'),(b'>one\r\naCgTu\r\n;ignored\r\nMrWsYkVhDbN\r\n','crlf-comments'),(('>oddwrap\n'+dna[:173]+'\n').encode(),'odd-wrap')]:check('revcomp',name,revcomp(data),data=data)
 for n in [1,21,101]:check('mandelbrot','odd-width-'+str(n),mandelbrot(n),n=n)
 data=b'>one\nagggtaaatttaccct\nAGGGTAAA\ntHaNtaNDcaNHaDWaSaNaSaNBY<x>|hello|\n'
 check('regexredux','case-and-replacements',regex(data),data=data)
 # A separately compiled D implementation supplies additional noncanonical small references.
 rows=[]
 for file in ['numerical-manifest.json','sequence-manifest.json','tree-digit-manifest.json']:rows+=json.loads((C/'d-comparison/20261011'/file).read_text())['programs']
 refs={r['benchmark']:r for r in rows if 'compact' in r['id']}
 for bench,ns in {'nbody':[0,1],'spectralnorm':[1,7],'fasta':[1,61],'binarytrees':[7],'pidigits':[1,9,27,101],'fannkuchredux':[1,2,5]}.items():
  for n in ns:
   r=refs[bench];p=subprocess.run([s.replace('{n}',str(n)) for s in r['command']],capture_output=True,timeout=30)
   assert p.returncode==0,(bench,n,p.stderr)
   expected=p.stdout
   if bench=='fannkuchredux' and n==2:expected=b'-1\nPfannkuchen(2) = 1\n'
   check(bench,'independent-D-'+str(n),expected,n=n)
 (OUT/'adversarial-validation.json').write_text(json.dumps(report,indent=2)+'\n')
 assert all(r['reference_match'] for r in report)
if __name__=='__main__':
 with open('/tmp/mech-clbg-timing.lock','a') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX)
  main()
