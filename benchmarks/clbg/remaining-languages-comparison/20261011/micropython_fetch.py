"""Preserve official sources for a native binary64 MicroPython UNIX runtime."""
import concurrent.futures
import importlib.util
import json
from pathlib import Path
import re
from urllib.request import urlopen
from dotnet_fetch import D,C,BASE,sha,parser,request
from metrics import normalize

def one(ident):
    p=D/'upstream/micropython'/(ident+'.html')
    if not p.exists():p.write_bytes(request(BASE+'program/'+ident+'.html').read())
    pre=parser.Preformatted();pre.feed(p.read_text())
    original=p.with_suffix('.py');original.write_text(pre.blocks[0].strip()+'\n')
    target=D/'sources/micropython'/original.name
    if not target.exists():target.write_bytes(original.read_bytes())
    return dict(id=ident,key=ident.split('-micropython-')[0]+':'+ident,benchmark=ident.split('-micropython-')[0],language='micropython',
        source=str(target),source_sha256=sha(target),original_source=str(original),original_source_sha256=sha(original),source_url=BASE+'program/'+ident+'.html',adaptations=[],local_adaptation=False,**normalize(target.read_text(),'micropython'))

def main():
    (D/'sources/micropython').mkdir(exist_ok=True)
    table=D/'upstream/micropython/measurements.html'
    names=sorted(set(re.findall(r'program/([^"<>]+)\.html',table.read_text())))
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(one,names))
    (D/'micropython_sources.json').write_text(json.dumps(dict(status='sources-pinned',programs=rows),indent=2)+'\n')
    print('MICROPYTHON PINNED',len(rows),flush=True)

if __name__=='__main__':main()
