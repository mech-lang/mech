"""Independent small canonical references; no full performance measurements."""
import hashlib
import json
import math
import re
import subprocess
from collections import Counter
from pathlib import Path

D = Path(__file__).resolve().parents[2]
PATTERNS = ['agggtaaa|tttaccct', '[cgt]gggtaaa|tttaccc[acg]',
            'a[act]ggtaaa|tttacc[agt]t', 'ag[act]gtaaa|tttac[agt]ct',
            'agg[act]taaa|ttta[agt]cct', 'aggg[acg]aaa|ttt[cgt]ccct',
            'agggt[cgt]aa|tt[acg]accct', 'agggta[cgt]a|t[acg]taccct',
            'agggtaa[cgt]|[acg]ttaccct']
SUBSTITUTIONS = [('tHa[Nt]', '<4>'), ('aND|caN|Ha[DS]|WaS', '<3>'),
                 ('a[NSt]|BY', '<2>'), ('<[^>]*>', '|'), (r'\|[^|][^|]*\|', '-')]
ALU = ('GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG'
       'GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA'
       'CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT'
       'ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA'
       'GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG'
       'AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC'
       'AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA')


def fasta(n):
    seed = 42
    result = []
    result.append('>ONE Homo sapiens alu\n')
    seq = (ALU * ((2 * n + len(ALU) - 1) // len(ALU)))[:2 * n]
    result.extend(seq[i:i+60] + '\n' for i in range(0, len(seq), 60))
    for name, length, alphabet, probs in [
        ('TWO IUB ambiguity codes', 3*n, 'acgtBDHKMNRSVWY', [0.27,0.12,0.12,0.27]+[0.02]*11),
        ('THREE Homo sapiens frequency', 5*n, 'acgt', [0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008])]:
        result.append('>'+name+'\n')
        cumul, total = [], 0.0
        for probability in probs:
            total += probability
            cumul.append(total)
        seq = []
        for i in range(length):
            seed = (seed * 3877 + 29573) % 139968
            r = float(seed) / 139968.0
            j = next(j for j, threshold in enumerate(cumul) if r < threshold)
            seq.append(alphabet[j])
        seq = ''.join(seq)
        result.extend(seq[i:i+60] + '\n' for i in range(0, len(seq), 60))
    return ''.join(result).encode()


def knucleotide(raw):
    seq, active = [], False
    for line in raw.decode().splitlines():
        if line.startswith('>'):
            if active:
                break
            active = line.startswith('>THREE')
        elif active and not line.startswith(';'):
            seq.append(line.upper())
    seq = ''.join(seq)
    tables = {k: Counter(seq[i:i+k] for i in range(len(seq)-k+1)) for k in [1,2,3,4,6,12,18]}
    result = ''
    for k in [1,2]:
        rows = sorted(tables[k].items(), key=lambda row: (-row[1], row[0]))
        result += ''.join(f'{key} {value*100.0/(len(seq)-k+1):.3f}\n' for key,value in rows)+'\n'
    result += ''.join(f'{tables[len(query)][query]}\t{query}\n' for query in ['GGT','GGTA','GGTATT','GGTATTTTAATT','GGTATTTTAATTTATAGT'])
    return result.encode()


def regex(raw):
    seq = re.sub(r'>[^\n]*\n|\n', '', raw.decode())
    result = ''.join(f'{pattern} {len(re.findall(pattern, seq))}\n' for pattern in PATTERNS)
    initial = len(seq)
    for pattern, replacement in SUBSTITUTIONS:
        seq = re.sub(pattern, replacement, seq)
    return (result+f'\n{len(raw)}\n{initial}\n{len(seq)}\n').encode()


def revcomp(raw):
    table = str.maketrans('ABCDGHKMNRSTVWYabcdghkmnrstvwy', 'TVGHCDMKNYSABWRtvghcdmknysabwr'.upper())
    out, header, sequence = [], None, []
    def emit():
        if header is not None:
            text = ''.join(sequence)[::-1].translate(table)
            out.append(header+'\n')
            out.extend(text[i:i+60]+'\n' for i in range(0,len(text),60))
    for line in raw.decode().splitlines():
        if line.startswith('>'):
            emit()
            header,sequence = line,[]
        elif not line.startswith(';'):
            sequence.append(line)
    emit()
    return ''.join(out).encode()


def reference(benchmark, n, raw=b''):
    if benchmark == 'fasta':
        return fasta(n)
    if benchmark == 'knucleotide':
        return knucleotide(raw)
    if benchmark == 'regexredux':
        return regex(raw)
    if benchmark == 'revcomp':
        return revcomp(raw)
    if benchmark == 'binarytrees':
        maxdepth = max(6,n)
        check = lambda d: (1 << (d+1))-1
        out = f'stretch tree of depth {maxdepth+1}\t check: {check(maxdepth+1)}\n'
        for depth in range(4,maxdepth+1,2):
            iterations = 1 << (maxdepth-depth+4)
            out += f'{iterations}\t trees of depth {depth}\t check: {iterations*check(depth)}\n'
        return (out+f'long lived tree of depth {maxdepth}\t check: {check(maxdepth)}\n').encode()
    if benchmark == 'pidigits':
        q,r,t,k = 1,0,1,0
        digits = []
        while len(digits) < n:
            k += 1
            r = (r+2*q)*(2*k+1)
            q *= k
            t *= 2*k+1
            if q > r:
                continue
            digit = (3*q+r)//t
            if digit != (4*q+r)//t:
                continue
            digits.append(str(digit))
            r = (r-digit*t)*10
            q *= 10
        digits = ''.join(digits)
        return ''.join(f'{digits[i:i+10]:10}\t:{min(i+10,n)}\n' for i in range(0,n,10)).encode()
    if benchmark == 'mandelbrot':
        out = bytearray(f'P4\n{n} {n}\n'.encode())
        for y in range(n):
            bits = 0
            ci = 2.0*y/n-1.0
            for x in range(n):
                cr = 2.0*x/n-1.5
                zr = zi = 0.0
                inside = True
                for iteration in range(50):
                    zr,zi = zr*zr-zi*zi+cr, 2.0*zr*zi+ci
                    if zr*zr+zi*zi > 4.0:
                        inside = False
                        break
                bits = (bits << 1) | int(inside)
                if x % 8 == 7:
                    out.append(bits)
                    bits = 0
            if n % 8:
                out.append(bits << (8-n%8))
        return bytes(out)
    if benchmark == 'fannkuchredux':
        perm = list(range(1,n+1))
        counts = [0]*n
        checksum = maximum = count = 0
        while True:
            a = perm.copy()
            flips = 0
            while a[0] != 1:
                k = a[0]
                a[:k] = a[:k][::-1]
                flips += 1
            checksum += flips if count % 2 == 0 else -flips
            maximum = max(maximum, flips)
            count += 1
            r = 2
            while r <= n:
                perm[:r] = perm[1:r]+perm[:1]
                counts[r-1] = (counts[r-1]+1)%r
                if counts[r-1]:
                    break
                r += 1
            if r > n:
                break
        return f'{checksum}\nPfannkuchen({n}) = {maximum}\n'.encode()
    if benchmark == 'spectralnorm':
        def a(i,j):
            return 1.0 / ((i+j)*(i+j+1)//2+i+1)
        def multiply(vector,transpose=False):
            return [sum((a(j,i) if transpose else a(i,j))*v for j,v in enumerate(vector)) for i in range(n)]
        def ata(vector):
            return multiply(multiply(vector),True)
        u=[1.0]*n
        for i in range(10):
            v=ata(u)
            u=ata(v)
        return f'{math.sqrt(sum(x*y for x,y in zip(u,v))/sum(x*x for x in v)):.9f}\n'.encode()
    if benchmark == 'nbody':
        sm = 4.0 * math.pi * math.pi
        bodies = [[0.0,0.0,0.0,0.0,0.0,0.0,sm],
                  [4.84143144246472090,-1.16032004402742839,-1.03622044471123109e-1,1.66007664274403694e-3,7.69901118419740425e-3,-6.90460016972063023e-5,9.54791938424326609e-4],
                  [8.34336671824457987,4.12479856412430479,-4.03523417114321381e-1,-2.76742510726862411e-3,4.99852801234917238e-3,2.30417297573763929e-5,2.85885980666130812e-4],
                  [1.28943695621391310e1,-1.51111514016986312e1,-2.23307578892655734e-1,2.96460137564761618e-3,2.37847173959480950e-3,-2.96589568540237556e-5,4.36624404335156298e-5],
                  [1.53796971148509165e1,-2.59193146099879641e1,1.79258772950371181e-1,2.68067772490389322e-3,1.62824170038242295e-3,-9.51592254519715870e-5,5.15138902046611451e-5]]
        for b in bodies[1:]:
            b[3:6] = [x*365.24 for x in b[3:6]]
            b[6] *= sm
        bodies[0][3:6] = [-sum(b[k]*b[6] for b in bodies)/sm for k in [3,4,5]]
        def energy():
            e=0.0
            for i,b in enumerate(bodies):
                e += 0.5*b[6]*sum(v*v for v in b[3:6])
                for c in bodies[i+1:]:
                    e -= b[6]*c[6]/math.sqrt(sum((b[k]-c[k])**2 for k in range(3)))
            return e
        initial=energy()
        for step in range(n):
            for i,b in enumerate(bodies):
                for c in bodies[i+1:]:
                    dx=[b[k]-c[k] for k in range(3)]
                    distance=math.sqrt(sum(x*x for x in dx))
                    mag=0.01/(distance*distance*distance)
                    for k in range(3):
                        b[k+3] -= dx[k]*c[6]*mag
                        c[k+3] += dx[k]*b[6]*mag
            for b in bodies:
                for k in range(3):
                    b[k] += 0.01*b[k+3]
        return f'{initial:.9f}\n{energy():.9f}\n'.encode()
    raise ValueError(benchmark)


def main():
    path = D/'smalltalk_manifest.json'
    manifest = json.loads(path.read_text())
    checks = D/'support/smalltalk/checks'
    checks.mkdir(exist_ok=True)
    dimensions = dict(nbody=1000,mandelbrot=32,knucleotide=1000,spectralnorm=30,revcomp=1000,
                      fannkuchredux=7,fasta=1000,binarytrees=10,regexredux=1000,pidigits=103)
    raw=fasta(1000)
    for row in manifest['programs']:
        if row['build_status'] != 'passed':
            continue
        n=dimensions[row['benchmark']]
        stdin=raw if row['benchmark'] in ['knucleotide','revcomp','regexredux'] else b''
        expected=reference(row['benchmark'],n,stdin)
        cmd=[arg.replace('{n}',str(n)) for arg in row['command']]
        process=subprocess.run(cmd,cwd=row['cwd'],input=stdin,capture_output=True,timeout=120)
        matched=process.returncode == 0 and process.stdout == expected
        row['small_validation']=dict(reference_match=matched,n=n,bytes=len(process.stdout),
                                     sha256=hashlib.sha256(process.stdout).hexdigest(),
                                     reference_sha256=hashlib.sha256(expected).hexdigest(),
                                     exit_code=process.returncode)
        (checks/(row['id']+'.stdout')).write_bytes(process.stdout)
        (checks/(row['id']+'.stderr')).write_bytes(process.stderr)
        (checks/(row['id']+'.reference')).write_bytes(expected)
        print('PASS' if matched else 'FAIL',row['id'],len(process.stdout),flush=True)
        if not matched:
            print(process.stderr.decode(errors='replace')[:3000],flush=True)
            if len(process.stdout)<3000:
                print('OUTPUT',repr(process.stdout), '\nREF',repr(expected),flush=True)
        path.write_text(json.dumps(manifest,indent=2)+'\n')
    manifest['status']='prepared' if all(r.get('small_validation',{}).get('reference_match') for r in manifest['programs']) else 'partial'
    path.write_text(json.dumps(manifest,indent=2)+'\n')


if __name__=='__main__':
    main()
