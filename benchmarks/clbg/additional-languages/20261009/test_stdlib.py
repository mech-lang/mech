"""Independent general-library checks, unrelated to benchmark inputs."""
import argparse,collections,json,random,re,subprocess
from pathlib import Path
D=Path(__file__).resolve().parent
ap=argparse.ArgumentParser(description=__doc__)
ap.add_argument('--binary',type=Path,default=D/'probes/stdlib-tests')
ap.add_argument('--output',type=Path,default=D/'results/stdlib-tests.json')
args=ap.parse_args()
rng=random.Random(20261009)
def limbs(x):
    x=abs(x);out=[]
    while x:out.append(x&0xffffffff);x>>=32
    return out or [0]
def data(x):return ('true' if x<0 else 'false')+' ['+', '.join(str(i)+'u32' for i in limbs(x))+']'
def value(sign,array):
    a=list(map(int,re.findall(r'(\d+)u32',array)))
    n=sum(x<<(32*i) for i,x in enumerate(a))
    return -n if sign=='true' else n
def call(entry,inputs):
    p=subprocess.run([str(args.binary),'--entry-point',entry],input=inputs,text=True,capture_output=True,timeout=10)
    assert p.returncode==0,p.stderr
    return p.stdout.strip().splitlines()
for i in range(160):
    a=rng.getrandbits(rng.randrange(1,257))*(-1 if rng.randrange(2) else 1)
    b=rng.getrandbits(rng.randrange(1,193))*(-1 if rng.randrange(2) else 1) or 1
    factor=rng.getrandbits(32)
    out=call('arithmetic',data(a)+' '+data(b)+' '+str(factor)+'u32')
    got=[value(out[j*2],out[j*2+1]) for j in range(4)]+[int(out[8].removesuffix('i64'))]
    expected=[a+b,a-b,a*factor,a//b,(a>b)-(a<b)]
    assert got==expected,(i,a,b,factor,got,expected)
print('160 random signed unbounded arithmetic cases passed')
cases=[('a*ab','aaabx','Z'),('a|aa','aaaa','!'),('[a-c]+','dcabbaXd','!'),('[^>]*>','xy>tail>','|'),(r'\|[^|][^|]*\|','a|one|b|two|','-'),('ab?c','abcacabbc','X'),('', 'abc','X'),('a*','baac','Y'),('tHa[Nt]','tHaNtHat','<4>'),(r'>.*\n|\n','>a\na\n>b\nb\n','')]
for _ in range(80):
    width=rng.randrange(1,7)
    branches=[''.join(rng.choice(['a','b','c','[ab]','[bc]','.']) for _ in range(width)) for _ in range(rng.randrange(1,5))]
    text=''.join(rng.choice('abcX\n') for _ in range(rng.randrange(0,80)))
    cases.append(('|'.join(branches),text,'!'))
def bytes_data(s):return '['+', '.join(str(c)+'u8' for c in s.encode())+']'
for pattern,text,replacement in cases:
    out=call('regex_test',bytes_data(pattern)+' '+bytes_data(text)+' '+bytes_data(replacement))
    count=int(out[0].removesuffix('i64'))
    actual=bytes(map(int,re.findall(r'(\d+)u8',out[1])))
    expected=re.sub(pattern,replacement,text).encode()
    expected_count=len(list(re.finditer(pattern,text)))
    assert (count,actual)==(expected_count,expected),(pattern,count,actual,expected_count,expected)
print(str(len(cases))+' regex feature/nonoverlap/substitution cases passed')
keys=[0,2**64-1]+[rng.randrange(512) for _ in range(4000)]
counts=collections.Counter(keys)
out=call('hashmap_test','['+', '.join(str(x)+'u64' for x in keys)+']')
assert list(map(int,re.findall(r'(\d+)i64',out[0])))==[counts[k] for k in keys]
print('Hash-map growth, collisions, zero and full-u64 keys passed')
args.output.write_text(json.dumps(dict(status='passed',arithmetic_cases=160,arithmetic_max_bits=256,regex_cases=len(cases),hashmap_updates=len(keys),random_seed=20261009),indent=2)+'\n')
