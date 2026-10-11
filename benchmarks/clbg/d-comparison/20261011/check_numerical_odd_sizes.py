"""Independent canonical Mandelbrot pixel oracle for non-byte-aligned sizes."""
import hashlib,json,pathlib,subprocess
D=pathlib.Path(__file__).resolve().parent
manifest=json.loads((D/'numerical-manifest.json').read_text())
def reference(n):
    result=bytearray(f'P4\n{n} {n}\n'.encode())
    for y in range(n):
        row=bytearray((n+7)//8)
        ci=2.0*y/n-1.0
        for x in range(n):
            cr=2.0*x/n-1.5
            zr=zi=0.0
            for _ in range(50):
                zi2=(zr+zr)*zi+ci
                zr=zr*zr-zi*zi+cr
                zi=zi2
                if zr*zr+zi*zi>4:break
            if zr*zr+zi*zi<=4:row[x//8]|=0x80>>(x%8)
        result.extend(row)
    return bytes(result)
rows=[]
for n in [21,101]:
    expected=reference(n)
    for r in manifest['programs']:
        if r['benchmark']!='mandelbrot':continue
        actual=subprocess.check_output([r['binary'],str(n)])
        assert actual==expected,(n,r['id'],hashlib.sha256(actual).hexdigest(),hashlib.sha256(expected).hexdigest())
        rows.append(dict(id=r['id'],n=n,bytes=len(actual),sha256=hashlib.sha256(actual).hexdigest(),reference_match=True))
        print('ODD SIZE PASS',r['id'],n,flush=True)
report=dict(status='passed',reference='Independent Python scalar double-precision implementation with canonical n-by-n coordinates, up to50 iterations, big-endian packed PBM bits and zero row tail bits.',checks=rows)
(D/'numerical-odd-size-audit.json').write_text(json.dumps(report,indent=2)+'\n')
for r in manifest['programs']:
    if r['benchmark']=='mandelbrot':r['odd_size_validation']=[v for v in rows if v['id']==r['id']]
(D/'numerical-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
