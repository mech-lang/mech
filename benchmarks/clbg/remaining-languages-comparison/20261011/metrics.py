"""Comment-free LOC; independent packed imperative statements count separately."""
import re
from pygments import lex
from pygments.lexers import get_lexer_by_name
from pygments.token import Comment, Literal

LEXERS = dict(csharp='csharp', fsharp='fsharp', ada='ada', pascal='delphi',
    fortran='fortran', dart='dart', smalltalk='smalltalk', micropython='python',
    toit='python')

def normalize(source, language):
    language = language.removesuffix('-aot').removesuffix('-jit')
    if language=='toit':
        import importlib.util
        from pathlib import Path
        spec=importlib.util.spec_from_file_location('toit_loc',Path(__file__).parent/'support/toit/loc_mask.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        cleaned=module.mask_toit(source)
    elif language=='smalltalk':
        token=re.compile(r'"[^"]*"|\x27(?:\x27\x27|[^\x27])*\x27|\$[^\n]',re.S)
        cleaned=token.sub(lambda m:'\n'*m[0].count('\n') if m[0].startswith('"')
            else re.sub(r'[^\r\n]+',lambda p:'S' if p[0].strip() else p[0],m[0]),source)
    else:
        tokens = lex(source.lstrip('\ufeff'),get_lexer_by_name(LEXERS[language],stripnl=False,ensurenl=False))
        cleaned = ''.join('\n'*s.count('\n') if t in Comment and t not in Comment.Preproc
            else re.sub(r'[^\r\n]+',lambda m:'S' if m[0].strip() else m[0],s) if t in Literal.String else s for t,s in tokens)
    physical = sum(bool(line.strip()) for line in cleaned.splitlines())
    if language=='fsharp':
        # Preserve multiline literals while masking their structural separators.
        depth=0
        chars=[]
        for ch in cleaned:
            if ch=='[':depth+=1
            chars.append(' ' if ch==';' and depth else ch)
            if ch==']':depth=max(0,depth-1)
        cleaned=''.join(chars)
    expansions=[]
    for number,line in enumerate(cleaned.splitlines(),1):
        if not line.strip():continue
        count=1
        if language in ('csharp','dart','ada','pascal','fortran'):
            # Semicolons in counted-loop headers and formal argument lists are structural.
            text=re.sub(r'\([^()]*(?:;[^()]*)+\)','()',line)
            count=max(1,text.count(';'))
            if language in ('csharp','dart'):
                # An initialized local declaration packed after a one-line function
                # or branch is a separate binding even without a final semicolon.
                assignments=len(re.findall(r'(?<![=!<>])(?:[+*/-]?=(?!=))(?!>)',text))
                if not re.search(r'\bfor\b',text):count=max(count,assignments)
        elif language=='fsharp':
            # List/array literals use structural semicolons; imperative sequencing does not.
            text=re.sub(r'\[\|?.*?\|?\]','LITERAL',line)
            count=max(1,text.count(';')+(bool(text.strip().rstrip(';')) if ';' in text else 0))
            count=max(count,len(re.findall(r'\b(?:let|and)\s+(?:mutable\s+)?\w+\s*=',text)))
        elif language=='smalltalk':
            parts=re.split(r';|(?<!\d)\.|\.(?!\d)',line)
            count=max(1,sum(bool(part.strip(' !\t')) for part in parts))
            count=max(count,line.count(':=')+int('^' in line))
        elif language in ('micropython','toit'):
            text=re.sub(r'\[[^\[\]]*\]','LITERAL',line)
            if language=='toit' and re.search(r'\bfor\b',text):text=text.split(';')[0]
            count=max(1,sum(bool(part.strip()) for part in text.split(';')))
            assignments=len(re.findall(r'(?<![=!<>])=(?!=)',text))
            if language=='micropython' and not re.search(r'\b(?:def|lambda)\b',text):
                count=max(count,assignments)
        if count>1:expansions.append(dict(line=number,extra_lines=count-1,
            reason='Independent statements or initialized bindings packed on one source line.'))
    return dict(physical_loc=physical,normalized_loc=physical+sum(x['extra_lines'] for x in expansions),expansions=expansions)

def fixtures():
    assert normalize('x=1; y=2;','csharp')['normalized_loc']==2
    assert normalize('for(int i=0;i<n;i++) f(i);','csharp')['normalized_loc']==1
    assert normalize('let a=[|1;2;3|]','fsharp')['normalized_loc']==1
    assert normalize('let a=[|1;2;\n3;4|]','fsharp')['normalized_loc']==2
    assert normalize('x <- 1; y <- 2','fsharp')['normalized_loc']==2
    assert normalize('X := 1; Y := 2;','ada')['normalized_loc']==2
    assert normalize('x := 1; y := 2;','pascal')['normalized_loc']==2
    assert normalize('x := 1. y := 2.','smalltalk')['normalized_loc']==2
    assert normalize('stream nextPut: x;\n    nextPut: y;\n    flush.','smalltalk')['normalized_loc']==3
    assert normalize('x="; # text" #comment','micropython')['normalized_loc']==1

if __name__=='__main__':fixtures();print('LOC fixtures passed')
