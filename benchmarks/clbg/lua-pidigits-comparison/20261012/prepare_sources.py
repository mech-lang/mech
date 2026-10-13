"""Formatting-only copies of the four pinned listed Lua application sources.

This stage neither builds nor executes nor hashes large artifacts. The locked
qualification stage must verify token-equivalence and the pinned upstream hashes.
"""
import re
from pathlib import Path
HERE=Path(__file__).resolve().parent
CLBG=HERE.parents[1]

def comma_parts(code):
    pieces=[];start=0;depth=0;quoted=None;escaped=False
    for i,char in enumerate(code):
        if quoted:
            if escaped:escaped=False
            elif char=='\\':escaped=True
            elif char==quoted:quoted=None
        elif char in ('"',"'"):quoted=char
        elif char in '({[':depth+=1
        elif char in ')}]':depth-=1
        elif char==',' and depth==0:pieces.append(code[start:i].strip());start=i+1
    pieces.append(code[start:].strip())
    return pieces

def normalize(text):
    result=[]
    for raw in text.splitlines():
        if raw.lstrip().startswith('--') or not raw.strip():result.append(raw);continue
        indent=raw[:len(raw)-len(raw.lstrip())];code=raw.strip()
        # All four reviewed programs use semicolons only to pack increment/if.
        statements=code.split(';')
        for statement in statements:
            code=statement.strip()
            if not code:continue
            matched=re.fullmatch(r'(if .+? then) (.+) end',code)
            if matched:
                result.extend([indent+matched[1],indent+'  '+matched[2],indent+'end']);continue
            matched=re.fullmatch(r'(local )?([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)+)\s*=\s*(.*)',code)
            if matched:
                prefix=matched[1] or '';names=comma_parts(matched[2]);values=comma_parts(matched[3]);assert len(names)==len(values),(code,names,values)
                result.extend(indent+prefix+name+' = '+value for name,value in zip(names,values));continue
            matched=re.fullmatch(r'local ([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)+)',code)
            if matched:
                result.extend(indent+'local '+name for name in comma_parts(matched[1]));continue
            result.append(indent+code)
    return '\n'.join(result)+'\n'

def main():
    for variant in (1,5,6,7):
        source=CLBG/'pidigits/upstream'/f'pidigits-lua-{variant}.lua'
        target=HERE/'sources'/f'pidigits-lua-{variant}-normalized.lua'
        target.write_text(normalize(source.read_text()))
        loc=sum(bool(line.strip()) and not line.lstrip().startswith('--') for line in target.read_text().splitlines())
        print(target.name,loc,'nonblank/noncomment lines; independent bindings/statements physically expanded')
if __name__=='__main__':main()
