"""D LOC: nonblank/noncomment lines, expanding packed independent statements."""
import re
from pygments import lex
from pygments.lexers import get_lexer_by_name
from pygments.token import Comment, Literal

def normalize(source):
    tokens=lex(source,get_lexer_by_name('d',stripnl=False,ensurenl=False))
    cleaned=''.join('\n'*s.count('\n') if t in Comment and t not in Comment.Preproc
        else re.sub(r'[^\r\n]+',lambda m:'S' if m[0].strip() else m[0],s) if t in Literal.String else s for t,s in tokens)
    physical=sum(bool(line.strip()) for line in cleaned.splitlines())
    chars=list(cleaned)
    for match in re.finditer(r'\b(?:for|foreach|foreach_reverse)\s*\(',cleaned):
        depth=0
        for i in range(match.end()-1,len(cleaned)):
            c=cleaned[i]
            if c=='(':depth+=1
            elif c==')':
                depth-=1
                if depth==0:break
            elif c==';' and depth==1:chars[i]=' '
    expansions=[]
    for number,line in enumerate(''.join(chars).splitlines(),1):
        if not line.strip():continue
        count=max(1,line.count(';'))
        if not re.search(r'\bfor\b',line):
            count=max(count,len(re.findall(r'(?<![=!<>])(?:[+*/%-]?=(?!=))(?!>)',line)))
        if count>1:expansions.append(dict(line=number,extra_lines=count-1,reason='Independent statements or initialized bindings packed on one source line.'))
    return dict(physical_loc=physical,normalized_loc=physical+sum(e['extra_lines'] for e in expansions),expansions=expansions)

def fixtures():
    assert normalize('x=1; y=2;')['normalized_loc']==2
    assert normalize('for(int i=0;i<n;i++) f(i);')['normalized_loc']==1
    assert normalize('foreach(i; a) f(i);')['normalized_loc']==1
    assert normalize('double x=0, y=0;')['normalized_loc']==2
    assert normalize('writeln("x;y"); // comment')['normalized_loc']==1
    assert normalize('/+ nested /+ comment +/ +/\nreturn x;')['normalized_loc']==1

if __name__=='__main__':fixtures();print('D LOC fixtures passed')
