"""Independent owned-integer, hash-map, regex and wrapping checks."""
import argparse,collections,hashlib,json,random,re,subprocess,sys
from pathlib import Path
D=Path(__file__).resolve().parent
ap=argparse.ArgumentParser(description=__doc__)
ap.add_argument('--binary',type=Path,default=D/'probes/general')
ap.add_argument('--output',type=Path,default=D/'results/stdlib-mojo-tests.json')
args=ap.parse_args()
sys.set_int_max_str_digits(20000)
rng=random.Random(20261009)
def call(*values):
    result=subprocess.run([str(args.binary),*map(str,values)],capture_output=True,timeout=10)
    assert result.returncode==0,result.stderr.decode()
    return result.stdout
integers=[(0,1,-2**63),(-2**63,-1,2**63-1),(2**63-1,2**63,0)]
for bits in (1,32,64,256,1024,4096,16384):
    for _ in range(16):
        a=rng.getrandbits(bits)*rng.choice((-1,1))
        b=rng.getrandbits(bits)*rng.choice((-1,1)) or 1
        factor=rng.randrange(-2**63,2**63)
        integers.append((a,b,factor))
for a,b,factor in integers:
    got=list(map(int,call('arithmetic',a,b,factor).splitlines()))
    unsigned_factor=factor & (2**64-1)
    fused=a*unsigned_factor+b
    assert got==[a+b,a-b,a*factor,a//b,a,a*7,fused,fused,fused,a*(unsigned_factor+1)],(a,b,factor,got)
print(str(len(integers))+' owned signed integer/operator/copy cases passed',flush=True)
keys=[0,2**64-1]+[rng.randrange(1024) for _ in range(4000)]+[rng.getrandbits(64) for _ in range(1000)]
counts=collections.Counter(keys)
got=list(map(int,call('map',*keys).splitlines()))
assert got==[counts[k] for k in keys]+[123,456]
print(str(len(keys))+' hash-map updates, growth and replacement passed',flush=True)
cases=[('a*ab','aaabx','Z'),('a|aa','aaaa','!'),('[a-c]+','dcabbaXd','longer replacement'),('[^>]*>','xy>tail>','|'),(r'\|[^|][^|]*\|','a|one|b|two|','-'),('ab?c','abcacabbc','X'),('', 'abc','X'),('a*','baac','Y'),('none','abc','larger'),(r'>.*\n|\n','>a\na\n>b\nb\n','')]
for _ in range(80):
    width=rng.randrange(1,7)
    branches=[''.join(rng.choice(['a','b','c','[ab]','[bc]','.']) for _ in range(width)) for _ in range(rng.randrange(1,5))]
    text=''.join(rng.choice('abcX\n') for _ in range(rng.randrange(0,80)))
    cases.append(('|'.join(branches),text,rng.choice(['','!','an expanded replacement'])))
for pattern,text,replacement in cases:
    first,actual=call('regex',pattern,text,replacement).split(b'\n',1)
    assert (int(first),actual)==(len(list(re.finditer(pattern,text))),re.sub(pattern,replacement,text).encode()),(pattern,text,replacement,first,actual)
print(str(len(cases))+' regex count and shrinking/expanding replacement cases passed',flush=True)
for _ in range(80):
    width=rng.randrange(1,80)
    text=''.join(rng.choice('abcXYZ\n') for _ in range(rng.randrange(0,300)))
    raw=text.encode()
    expected=b''.join(raw[j:j+width]+b'\n' for j in range(0,len(raw),width))
    assert call('wrap',width,text)==expected
print('80 independent wrapping cases passed',flush=True)
args.output.write_text(json.dumps(dict(status='passed',binary=str(args.binary.resolve()),binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest(),arithmetic_cases=len(integers),arithmetic_max_bits=16384,hashmap_updates=len(keys),regex_cases=len(cases),wrapping_cases=80,random_seed=20261009),indent=2)+'\n')
