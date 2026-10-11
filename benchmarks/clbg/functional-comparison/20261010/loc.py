"""Language-aware physical LOC plus packed, independent statements/bindings."""
import re
from pygments import lex
from pygments.lexers import get_lexer_by_name
from pygments.token import Comment,Literal
from fetch import clean_tokens
def normalize(source,language):
    tokens=list(lex(source,get_lexer_by_name(language,stripnl=False,ensurenl=False)))
    clean=clean_tokens(tokens,language)
    # Strings and comments cannot contain executable sequencing delimiters.
    structural=''.join(''.join('\n' if c=='\n' else ' ' for c in v) if t in Comment or t in Literal.String else v for t,v in tokens)
    physical=sum(bool(l.strip()) for l in clean.splitlines());extra={};stack=[]
    for n,line in enumerate(structural.splitlines(),1):
        i=0
        while i<len(line):
            c=line[i]
            if c in '[{':stack.append(c)
            elif c in ']}':
                if stack:stack.pop()
            elif c==';':
                run=2 if i+1<len(line) and line[i+1]==';' else 1
                remainder=line[i+run:].strip()
                if remainder and not remainder.startswith(('}',')','done','end',';')):
                    if language=='haskell' or not stack:extra[n]=extra.get(n,0)+1
                i+=run-1
            i+=1
        if language=='ocaml':
            # Multiple independent bindings on one physical line receive no discount.
            extra[n]=extra.get(n,0)+len(re.findall(r'\band\s+\w+\s*=',line))
    expansions=[dict(line=n,extra_lines=k,reason='Packed independent statements or bindings; structural collection/record separators excluded.',source=clean.splitlines()[n-1].strip()) for n,k in extra.items() if k]
    return dict(physical_loc=physical,normalized_loc=physical+sum(e['extra_lines'] for e in expansions),expansions=expansions)
