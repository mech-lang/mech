#!/usr/bin/env python3
"""Write LOC-only result tables with explicit measurement provenance."""
import json,statistics
from pathlib import Path
import run_remaining as r

def main():
    final_path=r.CLBG/'remaining-conformance/signed-division-confirmation.json'
    final=json.loads(final_path.read_text());assert all(len(p['samples_seconds'])==7 for p in final['programs'])
    summary=[]
    for benchmark in ('regexredux','pidigits'):
        path=r.CLBG/benchmark/'results/apple-m3-ac-normal-20261009.json';broad=json.loads(path.read_text())
        rows=[p for p in broad['programs'] if p['language']!='mech']
        for p in final['programs']:
            if p['benchmark']==benchmark:rows=[x for x in rows if x['id']!=p['id']]+[p]
        def loc(p):return p['source']['nonblank_noncomment_lines']
        lines=[f'# {benchmark}: latest measured endpoints','',
          'Apple M3; AC power; Low Power Mode off. Complete full workloads and stdout. '
          'Medians of three broad-sweep samples, replaced by seven-sample confirmations where available. '
          'These are the fastest and smallest **passing programs measured here**, not claims about unmeasured or excluded submissions.','',
          '| Language | Fastest passing program | Seconds | LOC | Smallest passing program | LOC | Seconds |',
          '| --- | --- | ---: | ---: | --- | ---: | ---: |']
        groups=[]
        for language in ('mech','c','cpp','rust','chapel','julia','go','java','python','ruby','perl','lua','luajit','javascript','php'):
            candidates=[p for p in rows if p['language']==language]
            if not candidates:continue
            fastest=min(candidates,key=lambda p:p['median_seconds']);smallest=min(candidates,key=lambda p:(loc(p),p['median_seconds']))
            lines.append(f'| {language} | {fastest["id"]} | {fastest["median_seconds"]:.6f} | {loc(fastest)} | {smallest["id"]} | {loc(smallest)} | {smallest["median_seconds"]:.6f} |')
            groups.append(dict(language=language,fastest=dict(id=fastest['id'],seconds=fastest['median_seconds'],loc=loc(fastest),samples=len(fastest['samples_seconds'])),smallest=dict(id=smallest['id'],seconds=smallest['median_seconds'],loc=loc(smallest),samples=len(smallest['samples_seconds']))))
        lines+=['','Complete pinned source LOC excludes blank/comment-only lines. Generic libraries and compiler machinery are excluded in every language. '
            'Java uses OpenJDK JIT for the last two tasks, rather than the upstream GraalVM native-image setup. '
            'LuaJIT uses unchanged Lua submissions with a compatible general library.','',
            'The broad report retains every build and validation exclusion. Its C++ pidigits #5 exclusion was resolved in the later confirmation by supplying the native-width GNU `ulong` alias on Darwin; its algorithm and source file are unchanged.','',
            'Latest confirmation: `../remaining-conformance/signed-division-confirmation.json`. '
            'Exact compiler/runtime/template snapshot is adjacent. Earlier broad snapshots additionally require '
            '`../remaining-conformance/baseline-application-templates.tar.gz`, retaining the templates omitted by their initial archive file filter.']
        (r.CLBG/benchmark/'results/latest-loc-table.md').write_text('\n'.join(lines)+'\n')
        summary.append(dict(benchmark=benchmark,endpoints=groups,broad_report_sha256=r.sha(path)))
    checks=json.loads((r.CLBG/'remaining-conformance/results.json').read_text())
    regressions=json.loads((r.CLBG/'remaining-conformance/regressions.json').read_text())
    output=dict(benchmarks=summary,confirmed_report_sha256=r.sha(final_path),
                general_feature_cases=len(checks['cases']),regression_checks=regressions['passed'],
                library_tests=33,compiler_admission_tests=2,
                limitations=['New standard-library operations currently target the experimental Cranelift AOT admission path.',
                  'The broad engine unit-test feature closure has pre-existing String: CanonicalCellBacking failures; the standalone native-features crate tests changed compiler APIs directly.',
                  'Compact runtimes have smallest-only endpoints in the ten-task aggregate because their fastest versions were not sampled for the earlier eight tasks.',
                  'Lua 5.5 pidigits output did not match the reference with its pinned bn binding; no ten-task Lua geometric mean is shown.'])
    (r.CLBG/'remaining-conformance/completion.json').write_text(json.dumps(output,indent=2)+'\n')
    print('Wrote complete LOC-only endpoint tables and validation summary')

if __name__=='__main__':main()
