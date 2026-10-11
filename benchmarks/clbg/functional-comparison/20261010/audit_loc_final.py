"""Finish LOC normalization without changing any measured source or sample."""
import json,re
from pathlib import Path
from pygments import lex
from pygments.lexers import get_lexer_by_name
from pygments.token import Comment,Literal
import run as runner
from fetch import clean_tokens
from loc import normalize
D=Path(__file__).resolve().parent;sha=runner.sha

def complete_metrics(source,language):
    initial=normalize(source,language)
    tokens=list(lex(source,get_lexer_by_name(language,stripnl=False,ensurenl=False)))
    clean=clean_tokens(tokens,language)
    structural=''.join(''.join('\n' if c=='\n' else ' ' for c in v) if t in Comment or t in Literal.String else v for t,v in tokens)
    expansions={e['line']:e.copy() for e in initial['expansions']}
    for number,line in enumerate(structural.splitlines(),1):
        lets=list(re.finditer(r'\blet\b',line));extra=0
        for index,match in enumerate(lets):
            prefix=line[:match.start()].rstrip()
            # A sequencing delimiter already counts this statement in the first audit.
            if prefix.endswith(';'):continue
            if index>0 or language=='haskell' and re.search(r'(?<![<=>/])=(?!=|>)',prefix):extra+=1
        if not extra:continue
        reason='Additional nested or adjacent let bindings on the same source line count separately.'
        previous=expansions.get(number)
        expansions[number]=dict(line=number,extra_lines=extra+(previous['extra_lines'] if previous else 0),
            reason=((previous['reason']+' ') if previous else '')+reason,source=clean.splitlines()[number-1].strip())
    result=dict(physical_loc=initial['physical_loc'],expansions=[expansions[n] for n in sorted(expansions)])
    result['normalized_loc']=result['physical_loc']+sum(e['extra_lines'] for e in result['expansions'])
    result['initial_normalized_loc']=initial['normalized_loc']
    return result

def main():
    # Nested and adjacent declarations cannot hide bindings; semicolon statements
    # receive one expansion, and structural collection separators remain neutral.
    assert complete_metrics('let f x = let y = x in y\n','ocaml')['normalized_loc']==2
    assert complete_metrics('let module M = C(struct let k = 1 let dna = x end) in M.run ()\n','ocaml')['normalized_loc']==3
    assert complete_metrics('let x = f (); let y = g ()\n','ocaml')['normalized_loc']==2
    assert complete_metrics('let xs = [1; 2; 3]\n','ocaml')['normalized_loc']==1
    assert complete_metrics('f x = let y = x in y\n','haskell')['normalized_loc']==2
    assert complete_metrics('f = do { foo; let x = 1; pure x }\n','haskell')['normalized_loc']==3
    assert complete_metrics('f = "let x = 1" -- let y = 2\n','haskell')['normalized_loc']==1
    path=D/'results/timings.json';report=json.loads(path.read_text())
    assert report['status']=='complete'
    rows={}
    for row in report['programs'].values():
        if row['language'] not in ('haskell','ocaml') or not row['eligible_for_chart']:continue
        source=Path(row['source']);assert sha(source)==row['source_sha256']
        metric=complete_metrics(source.read_text(),row['language'])
        assert metric['initial_normalized_loc']==row['loc']
        rows[row['key']]=dict(source_sha256=row['source_sha256'],**metric)
    result=dict(status='passed',timing_report_sha256=sha(path),source_audit_sha256=sha(D/'source-audit.json'),
        audit_script_sha256=sha(Path(__file__)),rows=rows,
        policy='Physical nonblank/noncomment lines, semantic pragmas included, plus independently packed sequencing statements, bindings, nested let bindings and adjacent module declarations. Structural collection separators and strings/comments are excluded. Source and timing hashes remain unchanged.')
    (D/'results/final-loc-audit.json').write_text(json.dumps(result,indent=2)+'\n')
    changed={key:dict(before=r['initial_normalized_loc'],after=r['normalized_loc']) for key,r in rows.items() if r['initial_normalized_loc']!=r['normalized_loc']}
    print(json.dumps(dict(audited=len(rows),adjusted=changed),indent=2))
if __name__=='__main__':main()
