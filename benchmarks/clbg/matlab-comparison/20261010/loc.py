"""MATLAB LOC: code lines plus packed independent statements, never compression."""
from pathlib import Path
from pygments import lex
from pygments.lexers import MatlabLexer
from pygments.token import Comment,String

def audit(path):
    source=Path(path).read_text()
    masked=''.join(''.join('\n' if c=='\n' else ' ' for c in value) if token in Comment or token in String else value
                   for token,value in lex(source,MatlabLexer()))
    original=source.splitlines();lines=masked.splitlines();physical=0;expansions=[];stack=[]
    for number,line in enumerate(lines,1):
        # Strings themselves are code, even if an otherwise empty line only contains a literal.
        raw=original[number-1] if number<=len(original) else ''
        if line.strip() or (raw.strip() and not raw.lstrip().startswith('%')):physical+=1
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
        if statements>1:expansions.append(dict(line=number,extra=statements-1,reason='Packed independent statements/control clauses',source=raw))
    return dict(physical_loc=physical,normalized_loc=physical+sum(e['extra'] for e in expansions),expansions=expansions,
                metric='statement-normalized',policy='Nonblank/noncomment physical lines, plus independent statements packed with semicolons/commas. Matrix/cell/record separators and strings are excluded. Semantic declarations and all local application helpers count. No compressed-source metric.')
