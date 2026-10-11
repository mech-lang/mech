"""Preserve published Erlang sources and separately pin required-work corrections."""
import hashlib
import json
from pathlib import Path

D = Path(__file__).resolve().parent


def sha(path):
    with Path(path).open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def replace(text, old, new):
    assert old in text, old
    return text.replace(old, new)


def main():
    manifest = json.loads((D / 'manifest.json').read_text())
    rows = []
    for row in manifest['programs']:
        text = Path(row['source']).read_text()
        original = text
        notes = []
        name, bench = row['id'], row['benchmark']
        if bench == 'binarytrees':
            text = replace(text, '-export([main/1]).', '-export([main/1, fresh_leaf/1]).')
            text = replace(text, 'bottomUp(0) -> {nil, nil};', 'bottomUp(0) -> ?MODULE:fresh_leaf(nil);')
            text += '\n% An exported call keeps the leaf tuple dynamic under BEAM optimization.\nfresh_leaf(Value) -> {Value, Value}.\n'
            notes.append('Allocate a fresh two-slot tuple at every depth-zero leaf instead of returning a shared literal. An ordinary exported Erlang constructor receives the two nil child values dynamically. Its code and export count toward LOC; optimized BEAM allocation is inspected before admission.')
        elif bench == 'fasta':
            if row['variant'] == 1:
                text = replace(text, '?IUB, N*3', 'cumulative(?IUB), N*3')
                text = replace(text, '?HS, N*5', 'cumulative(?HS), N*5')
                text = replace(text, 'choose_base(Bases, Prob - Freq).', 'choose_base(Bases, Prob).')
                text += '\n% Common cohort: cumulative binary64 probabilities and direct RNG division.\ncumulative(Probs) -> cumulative(Probs, 0.0).\ncumulative([], _) -> [];\ncumulative([{Base, P} | Rest], Sum) ->\n    Next = Sum + P,\n    [{Base, Next} | cumulative(Rest, Next)].\n'
                notes.append('Replace subtraction search with cumulative unscaled binary64 probabilities, retaining every LCG update and the original direct division per random value.')
            else:
                text = replace(text, '-define(PREC,10000000).\n', '')
                text = replace(text, '{trunc(NewSeed / ?IM * ?PREC), NewSeed}', '{NewSeed / ?IM, NewSeed}')
                text = replace(text, 'trunc(P*?PREC)', 'P')
                text = replace(text, '%Floats are expensive and we want to avoid dealing with floats in get_base/1.\n%Precalculate list of accumulated integers', '%Common cohort correction: unscaled cumulative binary64 probabilities.')
                notes.append('Remove integer-scaled probability lookup; each freshly generated LCG value is divided by IM and compared with cumulative binary64 probabilities. Retain original buffered output and RNG sequence.')
        elif bench == 'knucleotide':
            text = replace(text, 'lists:reverse(lists:keysort(2, ets:tab2list(FreqT)))', 'lists:sort(fun({A, CA}, {B, CB}) -> CA > CB orelse (CA == CB andalso A < B) end, ets:tab2list(FreqT))')
            notes.append('Break equal-frequency ties by ascending nucleotide key. Retain all seven complete overlapping ETS hash maps and every window update, including the final valid window.')
        elif bench == 'mandelbrot':
            if row['variant'] == 1:
                text = replace(text, 'when X =:= N-1', 'when X =:= N')
            else:
                text = replace(text, 'N-1, 0,', 'N, 0,')
            notes.append('Compute the final pixel of every row, rather than stopping at N-1 and relying on a zero padding bit. All pixels retain the ordinary at-most-fifty-iteration escape recurrence.')
            if row['variant'] in [1, 2]:
                text = replace(text, 'main([Arg]) ->\n', 'main([Arg]) ->\n    io:setopts(standard_io, [{encoding, latin1}]),\n')
                notes.append('Configure byte-oriented Latin-1 output so the PBM bytes are not UTF-8 encoded by standard_io. Header, packing and computation remain in the published Erlang source.')
        elif bench == 'regexredux':
            text = replace(text, 'Size = lists:sum(L),', 'All = iolist_to_binary([B1, B2, B3]),\n    Size = byte_size(All),')
            text = replace(text, 'matcher(S, B2, B3, MR)', 'matcher(S, All, MR)')
            text = replace(text, 'apply_final_patterns([B2,B3], final_patterns())', 'apply_final_patterns(All, final_patterns())')
            text = replace(text, 'FinalSize = size(B1) + size(Final)', 'FinalSize = size(Final)')
            text = replace(text, 'matcher(S, B2, B3, MR) ->', 'matcher(S, Data, MR) ->') if 'matcher(S, B2, B3, MR) ->' in text else text
            # The global replacement above also renames the definition's parameter.
            text = replace(text, 'countMatches(B2, MR) + countMatches(B3, MR)', 'countMatches(All, MR)')
            text = replace(text, '[{return,binary},caseless,global,never_utf]', '[{return,binary},global,never_utf]')
            text = replace(text, 'L = [size(X) || X <- [B1, B2, B3]],\n    ', '')
            notes.append('Run all nine count regexes and all five ordered substitutions over the entire cleaned input, including ALU and sequence boundaries. Preserve caseless counting but use the required case-sensitive substitutions. General Erlang re supplies regex operations; all sequencing and reporting remain in Erlang.')
        if text != original:
            adapted = row.copy()
            identifier = name + '-faithful'
            source = D / 'sources' / (identifier + '.erl')
            source.write_text(text)
            adapted.update(id=identifier, key=bench + ':' + identifier, source=str(source), source_sha256=sha(source),
                adaptations=notes, upstream_id=name, local_adaptation=True)
            rows.append(adapted)
    (D / 'corrections.json').write_text(json.dumps(dict(status='corrections-pinned',
        manifest_sha256=sha(D / 'manifest.json'), programs=rows), indent=2) + '\n')
    print('Pinned', len(rows), 'separate corrected sources')


if __name__ == '__main__':
    main()
