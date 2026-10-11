"""Untimed native conformance probes. Full benchmark timing belongs to the parent."""
from pathlib import Path
from collections import Counter
import hashlib, json, random, re, subprocess

S=Path(__file__).resolve().parent
D=S.parents[1]
C=D.parents[1]
M=D/'ada_pascal_manifest.json'
data=json.loads(M.read_text())
report=[]

def digest(content):return hashlib.sha256(content).hexdigest()

def check(row,name,expected,stdin=None,n=None):
    command=[arg.replace('{n}',str(n)) for arg in row['command']]
    try:result=subprocess.run(command,input=stdin,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=10)
    except subprocess.TimeoutExpired as exc:
        report.append(dict(id=row['id'],probe=name,passed=False,timeout_seconds=10,stdout=repr(exc.stdout),stderr=repr(exc.stderr)))
        print('AUDIT',row['id'],name,'TIMEOUT',flush=True)
        return
    ok=result.returncode==0 and result.stdout==expected
    rec=dict(id=row['id'],probe=name,passed=ok,bytes=len(result.stdout),sha256=digest(result.stdout),reference_sha256=digest(expected),returncode=result.returncode)
    if not ok:rec.update(stderr=result.stderr.decode(errors='replace')[:1000],actual=repr(result.stdout[:450]),expected=repr(expected[:450]))
    report.append(rec)
    print('AUDIT',row['id'],name,'PASS' if ok else 'FAIL',flush=True)

def koutput(dna):
    dna=dna.upper();widths=(1,2,3,4,6,12,18)
    maps={k:Counter(dna[i:i+k] for i in range(len(dna)-k+1)) for k in widths}
    text=''
    for k in (1,2):
        for key,value in sorted(maps[k].items(),key=lambda kv:(-kv[1],kv[0])):
            text+=f'{key} {100*value/(len(dna)-k+1):.3f}\n'
        text+='\n'
    for word in ('GGT','GGTA','GGTATT','GGTATTTTAATT','GGTATTTTAATTTATAGT'):
        text+=f'{maps[len(word)][word]}\t{word}\n'
    return text.encode(),{str(k):dict(distinct=len(v),windows=sum(v.values())) for k,v in maps.items()}

rng=random.Random(721_1818)
query='GGTATTTTAATTTATAGT'
kfixtures=[('case-crlf-boundaries','>ONE ignore\r\n'+query+'\r\n>THREE target\r\n',query.lower()+'AcgT'+query.lower(),'>FOUR exclude\r\n'+query*3+'\r\n'),
    ('full36bit-upper-key-collision','>THREE target\n',('AA'+query[2:])+'C'*21+query+'A'*19+query,'>FOUR\n'+query*5+'\n'),
    ('overlaps-ties','>THREE target\n','ACGT'*70+'GGGGTATTTTAATTTATAGTGGTA',''),
    ('map-growth','>THREE target\n',''.join(rng.choice('ACGT') for _ in range(20000)),''),
    ('absent-short','>THREE target\n','ACGTAC','')]
metadata=[]
for name,prefix,dna,suffix in kfixtures:
    ending='\r\n' if 'crlf' in name else '\n'
    stdin=(prefix+ending.join(dna[i:i+59] for i in range(0,len(dna),59))+ending+suffix).encode()
    expected,maps=koutput(dna)
    metadata.append(dict(name=name,stdin_sha256=digest(stdin),reference_sha256=digest(expected),maps=maps))
    (S/(name+'.fasta')).write_bytes(stdin)
    for row in data['programs']:
        if row['benchmark']=='knucleotide':check(row,name,expected,stdin=stdin)

pi_file=C/'pidigits/inputs/pidigits-output-small.txt'
pi=''.join(line.split('\t:')[0].strip() for line in pi_file.read_text().splitlines())
for n in (1,3,9,10,11,27,30):
    expected=''.join(pi[start:start+10][:n-start].ljust(10)+'\t:'+str(min(start+10,n))+'\n' for start in range(0,n,10)).encode()
    for row in data['programs']:
        if row['benchmark']=='pidigits':check(row,'partial-row-'+str(n),expected,n=n)

def mandel(n):
    bits=bytearray()
    for y in range(n):
        byte=0
        for x in range(n):
            cr=2*x/n-1.5;ci=2*y/n-1;zr=zi=0.0;inside=True
            for _ in range(50):
                zr,zi=zr*zr-zi*zi+cr,2*zr*zi+ci
                if zr*zr+zi*zi>4:inside=False;break
            byte=(byte<<1)|inside
            if x%8==7:bits.append(byte);byte=0
        if n%8:bits.append(byte<<(8-n%8))
    return f'P4\n{n} {n}\n'.encode()+bits
for n in (1,7,8,9,31,32):
    expected=mandel(n)
    for row in data['programs']:
        if row['benchmark']=='mandelbrot':check(row,'PBM-width-'+str(n),expected,n=n)

patterns=('agggtaaa|tttaccct','[cgt]gggtaaa|tttaccc[acg]','a[act]ggtaaa|tttacc[agt]t','ag[act]gtaaa|tttac[agt]ct','agg[act]taaa|ttta[agt]cct','aggg[acg]aaa|ttt[cgt]ccct','agggt[cgt]aa|tt[acg]accct','agggta[cgt]a|t[acg]taccct','agggtaa[cgt]|[acg]ttaccct')
regexdna=('agggtaaagggtaaaTTtttacccttttaccctcgggtaaaaaggtaaaagagtaaaaggataaaagggcaaaagggtcaaagggtacaaagggtaag'+'tHaNtHaTaNDcaNHaDHaSWaSaNaSaTBYS<hello>|abcd||Q|')*17
stdin=('>THREE synthetic\n'+regexdna+'\n').encode()
clean=re.sub(b'>[^\n]*\n|\n',b'',stdin)
expected=''.join(pattern+' '+str(len(re.findall(pattern.encode(),clean)))+'\n' for pattern in patterns).encode()
sub=clean
for pattern,replacement in ((b'tHa[Nt]',b'<4>'),(b'aND|caN|Ha[DS]|WaS',b'<3>'),(b'a[NSt]|BY',b'<2>'),(b'<[^>]*>',b'|'),(b'\\|[^|][^|]*\\|',b'-')):
    sub=re.sub(pattern,replacement,sub)
expected+=f'\n{len(stdin)}\n{len(clean)}\n{len(sub)}\n'.encode()
for row in data['programs']:
    if row['benchmark']=='regexredux':check(row,'patterns-order-nonoverlap',expected,stdin=stdin)

result=dict(probes=report,k_reference_maps=metadata,all_passed=all(r['passed'] for r in report))
(S/'adversarial-report.json').write_text(json.dumps(result,indent=2)+'\n')
print('COMPLETE',len(report),'probes,',sum(not r['passed'] for r in report),'failures',flush=True)
