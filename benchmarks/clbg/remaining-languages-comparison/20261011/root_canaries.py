"""Independent extra input checks for .NET and MicroPython; no times admitted."""
import collections
import json
import math
from pathlib import Path
import subprocess
from dotnet_fetch import D
from dotnet_prepare import ENV
from probe import save

def run(row,n=1,data=None):
    argv=[part.replace('{n}',str(n)) for part in row['command']]
    return subprocess.run(argv,input=data or b'',stdout=subprocess.PIPE,stderr=subprocess.PIPE,
        env=ENV|row.get('runtime_environment',{}),timeout=90,check=True).stdout

def nucleotide(sequence):
    text=sequence.upper()
    result=[]
    for width in [1,2]:
        counts=collections.Counter(text[i:i+width]for i in range(len(text)-width+1))
        for word,count in sorted(counts.items(),key=lambda p:(-p[1],p[0])):
            result.append('%s %.3f\n'%(word,100.0*count/(len(text)-width+1)))
        result.append('\n')
    for word in ['GGT','GGTA','GGTATT','GGTATTTTAATT','GGTATTTTAATTTATAGT']:
        counts=collections.Counter(text[i:i+len(word)]for i in range(len(text)-len(word)+1))
        result.append('%d\t%s\n'%(counts[word],word))
    return ''.join(result).encode()

def bitmap(n):
    data=bytearray(('P4\n%d %d\n'%(n,n)).encode())
    for y in range(n):
        row=bytearray((n+7)//8)
        for x in range(n):
            zr=zi=0.0
            cr=2.0*x/n-1.5
            ci=2.0*y/n-1.0
            inside=1
            for it in range(50):
                nr=zr*zr-zi*zi+cr
                zi=2.0*zr*zi+ci
                zr=nr
                if zr*zr+zi*zi>4.0:
                    inside=0;break
            row[x//8]|=inside<<(7-x%8)
        data.extend(row)
    return bytes(data)

def norm(n):
    def multiply(x,trans):
        return [sum(x[j]/((i+j)*(i+j+1)//2+(j if trans else i)+1) for j in range(n))for i in range(n)]
    def step(x):return multiply(multiply(x,False),True)
    u=[1.0]*n
    for it in range(10):
        v=step(u);u=step(v)
    return math.sqrt(sum(a*b for a,b in zip(u,v))/sum(b*b for b in v))

def pi(n):
    q,r,t,k=1,0,1,1
    result=[];line='';count=0
    while count<n:
        a=(3*q+r)//t;b=(4*q+r)//t
        if a==b:
            line+=str(a);count+=1
            if count%10==0 or count==n:
                result.append(line.ljust(10)+'\t:'+str(count)+'\n');line=''
            r=10*(r-a*t);q*=10
        else:r=(2*q+r)*(2*k+1);q*=k;t*=2*k+1;k+=1
    return ''.join(result).encode()

def regex(data):
    import re
    text=re.sub(rb'>.*\n|\n',b'',data)
    patterns=[b'agggtaaa|tttaccct',b'[cgt]gggtaaa|tttaccc[acg]',b'a[act]ggtaaa|tttacc[agt]t',b'ag[act]gtaaa|tttac[agt]ct',b'agg[act]taaa|ttta[agt]cct',b'aggg[acg]aaa|ttt[cgt]ccct',b'agggt[cgt]aa|tt[acg]accct',b'agggta[cgt]a|t[acg]taccct',b'agggtaa[cgt]|[acg]ttaccct']
    out=b''.join(p+b' '+str(len(re.findall(p,text))).encode()+b'\n'for p in patterns)
    result=text
    for p,r in [(b'tHa[Nt]',b'<4>'),(b'aND|caN|Ha[DS]|WaS',b'<3>'),(b'a[NSt]|BY',b'<2>'),(b'<[^>]*>',b'|'),(rb'\|[^|][^|]*\|',b'-')]:result=re.sub(p,r,result)
    return out+('\n%d\n%d\n%d\n'%(len(data),len(text),len(result))).encode()

def main():
    rows=json.loads((D/'dotnet_manifest.json').read_text())['programs']+json.loads((D/'micropython_manifest.json').read_text())['programs']
    failures=[];passed=[]
    for row in rows:
        benchmark=row['benchmark']
        cases=[]
        if benchmark=='knucleotide':
            seqs=['ACGT'*10,'GGTATTTTAATTTATAGT'*3+'AGGTATTTTAATTTATAGT'+'TGGTATTTTAATTTATAGT','ggtattacgtaCCGGttaa'*22,'AGGT']
            for sequence in seqs:
                data=('>ONE ignored\r\nGGT\r\n>THREE selected\r\n'+'\r\n'.join(sequence[i:i+7]for i in range(0,len(sequence),7))+'\r\n>FOUR ignored\r\nGGTATTTTAATTTATAGT\r\n').encode()
                cases.append((len(sequence),data,nucleotide(sequence)))
        elif benchmark=='mandelbrot':cases=[(n,None,bitmap(n))for n in [1,7,8,9,31]]
        elif benchmark=='pidigits':cases=[(n,None,pi(n))for n in [1,9,11,27]]
        elif benchmark=='spectralnorm':cases=[(n,None,norm(n))for n in [1,3,31]]
        elif benchmark=='regexredux':
            cases=[(1,data,regex(data))for data in [b'>ONE\nagggtaaatttaccct\naNDtHaNaStWaS\n',b'>x\r\nagggtaaa\r\n<test>BY|A||BB|tHaT\n',b'\n>ONE\n'+b'agggtaaa'*33+b'\n'+b'tHaNaND'*25+b'\n']]
        for n,data,expected in cases:
            try:
                actual=run(row,n,data)
                good=abs(float(actual)-expected)<1e-8 if benchmark=='spectralnorm' else actual==expected
                if not good:raise AssertionError((actual[:350],str(expected)[:350]))
                passed.append(dict(id=row['id'],n=n,benchmark=benchmark))
            except Exception as e:
                failure=dict(id=row['id'],n=n,benchmark=benchmark,error=str(e)[-800:]);failures.append(failure)
                print('FAIL',row['id'],n,str(e)[-500:],flush=True)
    save(D/'results/root-canaries.json',dict(status='passed' if not failures else 'needs-repair',passed=passed,failures=failures))
    print('CANARIES',len(passed),'passed;',len(failures),'failed',flush=True)

if __name__=='__main__':main()
