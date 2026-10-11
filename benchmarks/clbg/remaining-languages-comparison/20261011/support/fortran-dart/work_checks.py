"""Independent, untimed workload correctness and allocation checks."""
import argparse, collections, hashlib, json, os, random, re, subprocess, tempfile
from pathlib import Path

S=Path(__file__).resolve().parent
D=S.parents[1]
M=D/'fortran_dart_manifest.json'

def sha(path):
    with Path(path).open('rb') as file:return hashlib.file_digest(file,'sha256').hexdigest()

def execute(program,n=None,input_bytes=None,timeout=40):
    argv=[a.replace('{n}',str(n)) for a in program['command']]
    env=os.environ|{'OMP_NUM_THREADS':'8','OMP_DYNAMIC':'FALSE'}|program.get('runtime_environment',{})
    # Regular-file stdin matches the full measurement protocol; Fortran can
    # determine its size and still performs the entire read inside the process.
    with tempfile.TemporaryFile() as file:
        if input_bytes is not None:file.write(input_bytes);file.seek(0)
        result=subprocess.run(argv,input=None,stdin=file,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,timeout=timeout)
    assert result.returncode==0,(program['id'],result.returncode,result.stderr[:2000])
    return result.stdout

def k_reference(sequence):
    sequence=sequence.upper(); out=[]
    for width in [1,2,3,4,6,12,18]:
        counts=collections.Counter(sequence[i:i+width] for i in range(len(sequence)-width+1))
        if width<=2:
            out.extend(f'{key} {100.0*counts[key]/(len(sequence)-width+1):.3f}' for key in sorted(counts,key=lambda key:(-counts[key],key)))
            out.append('')
        else:
            target={3:'GGT',4:'GGTA',6:'GGTATT',12:'GGTATTTTAATT',18:'GGTATTTTAATTTATAGT'}[width]
            out.append(f'{counts[target]}\t{target}')
    return ('\n'.join(out)+'\n').encode()

def knucleotide(programs):
    rng=random.Random(20261011); target='GGTATTTTAATTTATAGT'
    random_dna=''.join(rng.choice('ACGT') for _ in range(20003))
    collisions=target+'CCCCCC'+('AA'+target[2:])+'TTTTTT'+('CC'+target[2:])+'CCCC'+target
    fixtures=[('upper36-bit-and-map-growth',random_dna+collisions,False),
        ('lowercase-crlf-and-record-boundary',(collisions+random_dna[:503]).lower(),True),
        ('ties-and-absent-long-windows','ACGTACGTACGT',False),
        ('one-base-and-empty-maps','A',False)]
    checks=[]
    for name,sequence,crlf in fixtures:
        separator='\r\n' if crlf else '\n'
        raw=('>ONE ignored'+separator+target*7+separator+'>THREE actual record'+separator+
            separator.join(sequence[i:i+61] for i in range(0,len(sequence),61))+separator+
            '>FOUR do not include'+separator+target*9+separator).encode()
        expected=k_reference(sequence)
        for program in programs:
            actual=execute(program,input_bytes=raw)
            assert actual==expected,(name,program['id'],actual[:400],expected[:400])
            checks.append(dict(id=program['id'],fixture=name,input_sha256=hashlib.sha256(raw).hexdigest(),source_sha256=sha(program['source']),binary_sha256=sha(program['binary']),output_sha256=hashlib.sha256(actual).hexdigest(),reference_match=True,formal_sample=False))
            print('K CANARY PASS',program['id'],name,flush=True)
    return checks

def mandel_reference(n):
    out=bytearray(f'P4\n{n} {n}\n'.encode())
    for y in range(n):
        row=bytearray((n+7)//8)
        for x in range(n):
            cr=2.0*x/n-1.5;ci=2.0*y/n-1.0;zr=zi=tr=ti=0.0
            for _ in range(50):
                zi=2.0*zr*zi+ci;zr=tr-ti+cr;tr=zr*zr;ti=zi*zi
                if tr+ti>4.0:break
            if tr+ti<=4.0:row[x//8]|=128>>(x%8)
        out.extend(row)
    return bytes(out)

def mandelbrot(programs):
    checks=[]
    for n in [1,7,13,31,33]:
        expected=mandel_reference(n)
        for program in programs:
            actual=execute(program,n=n)
            assert actual==expected,(n,program['id'],actual[:400],expected[:400])
            checks.append(dict(id=program['id'],n=n,source_sha256=sha(program['source']),binary_sha256=sha(program['binary']),output_sha256=hashlib.sha256(actual).hexdigest(),reference_match=True,formal_sample=False))
            print('PBM CANARY PASS',program['id'],n,flush=True)
    return checks

def main():
    parser=argparse.ArgumentParser();parser.add_argument('phase',choices=['knucleotide','mandelbrot']);args=parser.parse_args()
    report=json.loads(M.read_text());programs=[p for p in report['programs'] if p['benchmark']==args.phase and p.get('build_passed') and p.get('small_validation')]
    assert programs
    checks=globals()[args.phase](programs)
    out=S/(args.phase+'-work-checks.json')
    out.write_text(json.dumps(dict(status='passed',checks=checks),indent=2)+'\n')

if __name__=='__main__':main()
