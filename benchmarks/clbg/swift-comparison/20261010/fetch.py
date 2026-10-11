"""Pin every Swift program listed in CLBG 25.03, without gzip selection."""
import concurrent.futures, hashlib, importlib.util, json, re
from datetime import datetime, timezone
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urljoin
from urllib.request import Request, urlopen
from pygments import lex
from pygments.lexers import get_lexer_by_name
from pygments.token import Comment, Error, Punctuation

D=Path(__file__).resolve().parent
CLBG=D.parents[1]
BASE='https://benchmarksgame-team.pages.debian.net/benchmarksgame/'
spec=importlib.util.spec_from_file_location('upstream_parser',CLBG/'nbody/fetch_sources.py')
parser_module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(parser_module)
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def download(url):return urlopen(Request(url,headers={'User-Agent':'Mech benchmark source audit'}),timeout=60).read().decode()
def metrics(source):
    tokens=list(lex(source,get_lexer_by_name('swift',stripnl=False,ensurenl=False)))
    clean=''.join('\n'*value.count('\n') if token in Comment else value for token,value in tokens)
    return dict(physical_lines=len(source.splitlines()),nonblank_noncomment_lines=sum(bool(s.strip()) for s in clean.splitlines()),
                lexer_unclassified_tokens=[value for token,value in tokens if token in Error])
class Links(HTMLParser):
    def __init__(self):super().__init__();self.links=[]
    def handle_starttag(self,tag,attrs):
        if tag=='a':
            href=dict(attrs).get('href','')
            if re.search(r'/program/\w+-swift-\d+\.html$',href) and href not in self.links:self.links.append(href)
def fetch(href):
    url=urljoin(BASE+'measurements/swift.html',href)
    identifier=Path(url).stem
    benchmark=identifier.split('-swift-')[0]
    raw=download(url);(D/'upstream'/(identifier+'.html')).write_text(raw)
    parser=parser_module.Preformatted();parser.feed(raw)
    assert len(parser.blocks)>=2,identifier
    source=parser.blocks[0].strip()+'\n'
    original=D/'upstream'/(identifier+'.swift');original.write_text(source)
    adapted=source
    changes=[]
    if 'import Glibc' in adapted:
        adapted=adapted.replace('import Glibc','import Darwin')
        changes.append('Replace Linux Glibc import with the corresponding macOS Darwin module; no algorithm change.')
    # Split independent Swift statements at semicolons outside strings/comments.
    tokens=list(lex(adapted,get_lexer_by_name('swift',stripnl=False,ensurenl=False)))
    if any(token in Punctuation and ';' in value for token,value in tokens):
        adapted=''.join(value.replace(';','\n') if token in Punctuation else value for token,value in tokens)
        changes.append('Expand semicolon-separated statements onto separate lines for the established LOC rule.')
    target=D/'sources'/(identifier+'.swift');target.write_text(adapted)
    return dict(id=identifier,key=benchmark+':'+identifier,benchmark=benchmark,language='swift',variant=int(identifier.rsplit('-',1)[1]),
                source=str(target),source_sha256=sha(target),original_source=str(original),original_source_sha256=sha(original),
                page_sha256=sha(D/'upstream'/(identifier+'.html')),source_url=url,upstream_notes=parser.blocks[1].strip(),
                source_metrics=metrics(adapted),original_source_metrics=metrics(source),adaptations=changes)
def main():
    path=D/'manifest.json'
    assert not path.exists(),'Source manifest is already pinned.'
    for directory in ('upstream','sources','results','binaries'):(D/directory).mkdir(exist_ok=True)
    table=D/'upstream/swift-measurements.html'
    if not table.exists():table.write_text(download(BASE+'measurements/swift.html'))
    links=Links();links.feed(table.read_text())
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:rows=list(pool.map(fetch,links.links))
    license_path=D/'upstream/LICENSE.txt'
    license_parser=parser_module.Preformatted();license_parser.feed(download(BASE+'license.html'))
    license_text='\n'.join(t.strip() for t in license_parser.all_text if t.strip())+'\n'
    license_path.write_text(license_text[license_text.index('Revised BSD license'):])
    report=dict(status='sources-pinned',retrieved_at=datetime.now(timezone.utc).isoformat(),table_url=BASE+'measurements/swift.html',
                table_sha256=sha(table),selection='Every Swift source linked from the CLBG 25.03 Swift measurements page, including listed failures. Eligibility requires local compilation, prescribed work, and small/full output validation. Select endpoints by locally measured time and nonblank/noncomment LOC, never gzip.',
                programs=rows,license_sha256=sha(license_path))
    path.write_text(json.dumps(report,indent=2)+'\n')
    for row in rows:print(row['id'],row['source_metrics']['nonblank_noncomment_lines'],'LOC',len(row['adaptations']),'adaptations',flush=True)
    print('PINNED',len(rows),'sources across',len({r['benchmark'] for r in rows}),'benchmarks')
if __name__=='__main__':main()
