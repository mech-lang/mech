#!/usr/bin/env python3
"""Independent small/adversarial correctness, map and allocation audits only."""
from pathlib import Path
import subprocess,os,json,hashlib,random,collections,re,sys
G=Path(__file__).resolve().parent
CLBG=G.parents[1]
R=G/'.build/toolchains/R-4.6.1/bin/Rscript'
env=os.environ|{'R_LIBS_USER':str(G/'.build/toolchains/r-library'),'R_ENABLE_JIT':'3','OMP_NUM_THREADS':'1','OPENBLAS_NUM_THREADS':'1'}
RESULTS=G/'results/r'
selected=set(sys.argv[1].split(",")) if len(sys.argv)>1 else None
report=[]
d=[]
for f in ['sequence-manifest.json','numerical-manifest.json','tree-digit-manifest.json']:
 d+=json.loads((CLBG/'d-comparison/20261011'/f).read_text())['programs']
references={r['benchmark']:r for r in d if 'compact' in r['id']}
def execute(bench,variant,n=None,data=None):
 p=G/'sources/r'/bench/(variant+'.R')
 return subprocess.run([str(R),'--vanilla',str(p)]+([] if n is None else [str(n)]),input=data,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,timeout=90)
def check(bench,case,expected,n=None,data=None):
 if selected is not None and bench not in selected:return
 for variant in ['compact','optimized']+(['array-compact','array-batched'] if bench=='fannkuchredux' else []):
  p=execute(bench,variant,n,data)
  ok=p.returncode==0 and p.stdout==expected
  report.append({'benchmark':bench,'variant':variant,'case':case,'n':n,'input_sha256':hashlib.sha256(data).hexdigest() if data is not None else None,'bytes':len(p.stdout),'sha256':hashlib.sha256(p.stdout).hexdigest(),'reference_match':ok,'returncode':p.returncode,'stderr':p.stderr.decode()})
  print(bench,variant,case,'PASS' if ok else 'FAIL',p.stderr.decode()[:500],flush=True)
  if not ok:print('actual',repr(p.stdout[:500]),'expected',repr(expected[:500]),flush=True)
def knuc(data):
 reading=False
 seq=[]
 for line in data.decode().splitlines():
  if line.startswith('>THREE'):reading=True
  elif reading and line.startswith('>'):break
  elif reading and not line.startswith(';'):seq.append(line.upper())
 seq=''.join(seq)
 out=[]
 queries={3:'GGT',4:'GGTA',6:'GGTATT',12:'GGTATTTTAATT',18:'GGTATTTTAATTTATAGT'}
 for width in [1,2,3,4,6,12,18]:
  counts=collections.Counter(seq[i:i+width] for i in range(max(0,len(seq)-width+1)))
  if width <= 2:
   out.extend(f'{key} {100*count/sum(counts.values()):.3f}\n' for key,count in sorted(counts.items(),key=lambda item:(-item[1],item[0])))
   out.append('\n')
  else:out.append(f'{counts[queries[width]]}\t{queries[width]}\n')
 return ''.join(out).encode()
def revcomp(data):
 pairs=str.maketrans('ACGTUMRWSYKVHDBNacgtumrwsykvhdbn','TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN')
 records=[]
 for line in data.decode().splitlines():
  if line.startswith('>'):records.append([line,[]])
  elif records and not line.startswith(';'):records[-1][1].append(line)
 out=[]
 for head,seq in records:
  text=''.join(seq).translate(pairs)[::-1]
  out.append(head+'\n')
  out.extend(text[i:i+60]+'\n' for i in range(0,len(text),60))
 return ''.join(out).encode()
for data,name in [(b'>ONE ignore\nACGT\n>THREE ties\nAAAAACCCCGGGGTTTT\n>FOUR stop\nGGGG\n','lexical-ties-stop'),(b'>THREE short\nA\n','short-window'),(b'>THREE empty\n>FOUR\nAGG\n','empty-record'),(b'>ONE\nACGT\n','missing-record'),(b'>THREE crlf\r\naCgT\r\n;ignored comment\r\nGGTATT\r\n','mixed-case-comments-crlf')]:check('knucleotide',name,knuc(data),data=data)
rng=random.Random(20261011)
dna=''.join(rng.choice('ACGT') for _ in range(10003))+'GGTATTTTAATTTATAGT'
data=('>THREE arbitrary\n'+'\n'.join(dna[i:i+37].lower() for i in range(0,len(dna),37))+'\n').encode()
check('knucleotide','arbitrary-all-widths',knuc(data),data=data)
for data,name in [(b'>one\nACBDGHKMNSRUTWVYacbdghkmnsrutwvy\n>empty\n>two\nAACCGGTT\n','all-IUB-and-empty'),(b'>one\r\naCgTu\r\n;ignored\r\nMrWsYkVhDbN\r\n','crlf-comments'),(('>oddwrap\n'+dna[:173]+'\n').encode(),'non-sixty-record')]:check('revcomp',name,revcomp(data),data=data)
for bench,values in {'fasta':[1,2,61,20000],'pidigits':[1,9,11,37,101],'fannkuchredux':[1,2,3,4,5,6,8,9],'spectralnorm':[1,2,7,113],'nbody':[0,1,10000],'binarytrees':[4,7]}.items():
 for n in values:
  r=references[bench]
  command=[s.replace('{n}',str(n)) for s in r['command']]
  p=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=90)
  assert p.returncode==0,(bench,n,p.stderr)
  expected=p.stdout
  if bench=='fannkuchredux' and n==2:expected=b'-1\nPfannkuchen(2) = 1\n'
  check(bench,'signed-checksum-'+str(n) if bench=='fannkuchredux' and n==2 else 'independent-D-'+str(n),expected,n=n)
patterns=['agggtaaa|tttaccct','[cgt]gggtaaa|tttaccc[acg]','a[act]ggtaaa|tttacc[agt]t','ag[act]gtaaa|tttac[agt]ct','agg[act]taaa|ttta[agt]cct','aggg[acg]aaa|ttt[cgt]ccct','agggt[cgt]aa|tt[acg]accct','agggta[cgt]a|t[acg]taccct','agggtaa[cgt]|[acg]ttaccct']
data=b'>one\nagggtaaatttaccct\nAGGGTAAA\ntHaNtaNDcaNHaDWaSaNaSaNBY<x>|hello|\n'
cleaned=re.sub(b'>[^\n]*\n|\n',b'',data)
text=cleaned
for pattern,replacement in [(b'tHa[Nt]',b'<4>'),(b'aND|caN|Ha[DS]|WaS',b'<3>'),(b'a[NSt]|BY',b'<2>'),(b'<[^>]*>',b'|'),(rb'\|[^|][^|]*\|',b'-')]:text=re.sub(pattern,replacement,text)
expected=(''.join(f'{p} {len(re.findall(p.encode(),cleaned))}\n' for p in patterns)+f'\n{len(data)}\n{len(cleaned)}\n{len(text)}\n').encode()
check('regexredux','case-sensitive-and-all-replacements',expected,data=data)
# The bitmap reference independently evaluates real recurrences and packs padded rows.
for n in [1,3,7,9,17,65]:
 bits=[]
 for y in range(n):
  row=[]
  for x in range(n):
   zr=zi=0.0
   inside=True
   for it in range(50):
    zr,zi=zr*zr-zi*zi+2*x/n-1.5,2*zr*zi+2*y/n-1
    if zr*zr+zi*zi>4:inside=False;break
   row.append(inside)
  row+=[False]*((-n)%8)
  bits.extend(sum(int(b)*(1<<(7-j)) for j,b in enumerate(row[i:i+8])) for i in range(0,len(row),8))
 check('mandelbrot','padding-'+str(n),f'P4\n{n} {n}\n'.encode()+bytes(bits),n=n)
previous=json.loads((RESULTS/'adversarial-validation.json').read_text()) if selected is not None else []
previous=[r for r in previous if r['benchmark'] not in selected] if selected is not None else []
report=previous+report
(RESULTS/'adversarial-validation.json').write_text(json.dumps(report,indent=2)+'\n')
assert all(r['reference_match'] for r in report)
