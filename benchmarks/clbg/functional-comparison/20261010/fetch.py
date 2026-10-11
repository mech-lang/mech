"""Pin the complete published Haskell/OCaml candidate pools and measured-workload hints."""
import concurrent.futures,hashlib,importlib.util,json,re
from datetime import datetime,timezone
from html.parser import HTMLParser
from html import unescape
from pathlib import Path
from urllib.request import Request,urlopen
from urllib.parse import urljoin
from pygments import lex
from pygments.lexers import get_lexer_by_name
from pygments.token import Comment,Error,Punctuation
D=Path(__file__).resolve().parent
CLBG=D.parents[1]
BASE='https://benchmarksgame-team.pages.debian.net/benchmarksgame/'
LANGUAGES={'ghc':('haskell','.hs'),'ocaml':('ocaml','.ml')}
spec=importlib.util.spec_from_file_location('upstream_parser',CLBG/'nbody/fetch_sources.py')
parser_module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(parser_module)
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def download(url):return urlopen(Request(url,headers={'User-Agent':'Mech benchmark source audit'}),timeout=60).read().decode()
def metrics(source,language):
 tokens=list(lex(source,get_lexer_by_name(language,stripnl=False,ensurenl=False)))
 clean=clean_tokens(tokens,language)
 lines=sum(bool(s.strip()) for s in clean.splitlines())
 # Packed imperative sequencing has no LOC discount. Its syntax is retained;
 # semicolon-separated independent statements count separately even when packed.
 additions=0;expansions=[]
 for number,line in enumerate(clean.splitlines(),1):
  line_tokens=list(lex(line,get_lexer_by_name(language,stripnl=False,ensurenl=False)))
  semis=sum(value.count(';') for token,value in line_tokens if token in Punctuation and ';' in value)
  if language=='ocaml':semis=sum(value.count(';') for token,value in line_tokens if token not in Comment and str(token).startswith(('Token.Operator','Token.Punctuation')))
  # OCaml list literals also use semicolons; these are structural, not statements.
  # The preparation audit replaces this conservative raw hint with reviewed LOC.
  if semis:expansions.append(dict(line=number,semicolons=semis,source=line.strip()))
 return dict(physical_lines=len(source.splitlines()),nonblank_noncomment_lines=lines,packed_statement_review=expansions,lexer_unclassified_tokens=[v for t,v in tokens if t in Error])
def clean_tokens(tokens,language):
 result=[];pragma=False
 for i,(token,value) in enumerate(tokens):
  if language=='haskell' and token in Comment and value=='{-' and i+1<len(tokens) and tokens[i+1][1].startswith('#'):pragma=True
  result.append(value if token not in Comment or pragma else '\n'*value.count('\n'))
  if pragma and value.endswith('-}'):pragma=False
 return ''.join(result)
def listed(table,upstream):
 result={}
 for row in re.split(r'<tr\b[^>]*>',table)[1:]:
  match=re.search(r'program/(\w+-'+upstream+r'-\d+)\.html',row)
  if not match:continue
  identifier=match[1]
  cells=[unescape(re.sub(r'<[^>]*>','',s)).strip() for s in re.split(r'<td\b[^>]*>',row)[1:]]
  try:elapsed=float(re.match(r'[\d.,]+',cells[2])[0].replace(',',''));work=int(re.match(r'[\d,]+',cells[6])[0].replace(',',''))
  except (ValueError,IndexError):elapsed=work=None
  record=result.setdefault(identifier,dict(id=identifier,href='../program/'+identifier+'.html',published=[]))
  record['published'].append(dict(elapsed=elapsed,n=work))
 return list(result.values())
def fetch(item):
 upstream=item['upstream'];language,suffix=LANGUAGES[upstream]
 url=urljoin(BASE+'measurements/'+upstream+'.html',item['href'])
 raw=download(url);identifier=item['id']
 page=D/'upstream'/(identifier+'.html');page.write_text(raw)
 parser=parser_module.Preformatted();parser.feed(raw)
 assert len(parser.blocks)>=2,identifier
 original=D/'upstream'/(identifier+suffix);original.write_text(parser.blocks[0].strip()+'\n')
 target=D/'sources'/(identifier+suffix);target.write_bytes(original.read_bytes())
 return dict(id=identifier,key=identifier.split('-'+upstream+'-')[0]+':'+identifier,benchmark=identifier.split('-'+upstream+'-')[0],language=language,upstream_language=upstream,variant=int(identifier.rsplit('-',1)[1]),source=str(target),source_sha256=sha(target),original_source=str(original),original_source_sha256=sha(original),page_sha256=sha(page),source_url=url,upstream_notes=parser.blocks[1].strip(),published=item['published'],source_metrics=metrics(target.read_text(),language),adaptations=[])
def main():
 assert not (D/'manifest.json').exists()
 for name in ('upstream','sources','binaries','builds','results','support'):(D/name).mkdir(exist_ok=True)
 items=[];tables={}
 for upstream in LANGUAGES:
  url=BASE+'measurements/'+upstream+'.html';p=D/'upstream'/(upstream+'-measurements.html')
  p.write_text(download(url));tables[upstream]=dict(url=url,sha256=sha(p))
  for item in listed(p.read_text(),upstream):item['upstream']=upstream;items.append(item)
 with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:rows=list(pool.map(fetch,items))
 license_parser=parser_module.Preformatted();license_parser.feed(download(BASE+'license.html'))
 text='\n'.join(t.strip() for t in license_parser.all_text if t.strip())+'\n'
 p=D/'upstream/LICENSE.txt';p.write_text(text[text.index('Revised BSD license'):])
 report=dict(status='sources-pinned',retrieved_at=datetime.now(timezone.utc).isoformat(),tables=tables,programs=rows,license_sha256=sha(p),selection='Every source on the published Haskell GHC and OCaml measurements pages. Published elapsed times are candidate-selection hints only, never local timings. LOC excludes blanks/comments and separately audits packed statements; no compressed-size selection.')
 (D/'manifest.json').write_text(json.dumps(report,indent=2)+'\n')
 for row in rows:print(row['id'],row['source_metrics']['nonblank_noncomment_lines'],'lines',flush=True)
 print('PINNED',len(rows),'sources',dict((l,sum(r['language']==l for r in rows)) for l,_ in LANGUAGES.values()))
if __name__=='__main__':main()
