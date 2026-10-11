"""Recheck measured MATLAB statement LOC without changing sources or timings."""
import hashlib,json
from pathlib import Path
from pygments import lex
from pygments.lexers import MatlabLexer
from pygments.token import Comment,String
from prepare import sha
D=Path(__file__).resolve().parent
def metrics(source):
    chunks=[]
    for token,value in lex(source,MatlabLexer(stripnl=False,ensurenl=False)):
        if token in Comment:chunks.append(''.join('\n' if c=='\n' else ' ' for c in value))
        elif token in String:
            blank=''.join('\n' if c=='\n' else ' ' for c in value)
            chunks.append(('S'+blank[1:]) if blank else blank)
        else:chunks.append(value)
    lines=''.join(chunks).splitlines();original=source.splitlines();physical=0;expansions=[];stack=[]
    for number,line in enumerate(lines,1):
        if line.strip():physical+=1
        statements=0;segment=False
        for c in line:
            if c in '([{':stack.append(c);segment=True
            elif c in ')]}':
                if stack:stack.pop()
                segment=True
            elif c in ';,' and not stack:
                if segment:statements+=1
                segment=False
            elif not c.isspace():segment=True
        if segment:statements+=1
        if statements>1:expansions.append(dict(line=number,extra_lines=statements-1,reason='Packed independent statements/control clauses',source=original[number-1]))
    return dict(physical_loc=physical,normalized_loc=physical+sum(e['extra_lines'] for e in expansions),expansions=expansions)
def main():
    assert metrics('x=1; y=2;\n')['normalized_loc']==2
    assert metrics('x=[1,2;3,4];\n')['normalized_loc']==1
    assert metrics("x='a;b'; % c;d\n")['normalized_loc']==1
    assert metrics('x={1,2;3,4};\n')['normalized_loc']==1
    assert metrics('%{\na;b\n%}\nx=1;\n')['normalized_loc']==1
    path=D/'results/timings.json';raw=path.read_bytes();report=json.loads(raw);assert report['status']=='complete'
    rows={}
    for row in report['programs'].values():
        if row['language']!='matlab':continue
        assert sha(row['source'])==row['source_sha256']
        metric=metrics(Path(row['source']).read_text());assert metric['normalized_loc']==row['loc'] and metric['physical_loc']==row['physical_loc']
        rows[row['key']]=dict(source_sha256=row['source_sha256'],**metric)
    assert len(rows)==20
    result=dict(status='passed',timing_report_sha256=hashlib.sha256(raw).hexdigest(),source_audit_sha256=sha(D/'source-audit.json'),audit_script_sha256=sha(Path(__file__)),rows=rows,
        policy='Nonblank/noncomment physical MATLAB lines plus packed independent statements/control clauses. Matrix/cell separators and string/comment contents are excluded. Every application helper, stream open, declaration and local correction counts. Standard libraries and the stream-only launcher are excluded; no compression metric.')
    (D/'results/final-loc-audit.json').write_text(json.dumps(result,indent=2)+'\n');print('Verified twenty measured MATLAB source LOC counts.')
if __name__=='__main__':main()
