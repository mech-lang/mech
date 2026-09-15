#!/usr/bin/env python3
"""Verify coverage accounting, not implementation readiness."""
import csv,json,pathlib,re
D=pathlib.Path(__file__).resolve().parent
R=D.parents[3]
def tsv(p):return list(csv.DictReader(p.open(),delimiter='\t'))
def exact(rows,key,expected):
 actual=[r[key] for r in rows]
 assert len(actual)==len(set(actual)),f'duplicates in {key}'
 assert set(actual)==set(expected),(key,set(actual)^set(expected))
gaps=tsv(D/'gaps.tsv');exact(gaps,'gap-id',[f'G{i:02}' for i in range(1,25)])
cases=json.loads((R/'tests/fixtures/s8-replacement-audit/semantic-cases.json').read_text())
sem=tsv(D/'semantic-obligations.tsv');exact(sem,'case-id',[x['id'] for x in cases])
assert len(sem)==365
observations=json.loads((D/'observations.json').read_text())
obs=[r['value'] for r in observations if r['record']=='AUDIT'];exact(obs,'id',[x['id'] for x in cases])
for r in sem:
 if r['observed-stage']!='pass':assert r['gap-id'],r
 if r['gap-id']:assert r['gap-id'] in {x['gap-id'] for x in gaps}
 if r['oracle']=='source-bytecode-equivalence-only':assert r['open-obligation']=='O03'
assert sum(r['observed-stage']=='pass' for r in sem)==317
for name,count,key,source,sourcekey in [
 ('consumer-contracts.tsv',27,'consumer-id','source-parser-consumers.tsv','consumer-id'),
]:
 rows=tsv(D/name);assert len(rows)==count;exact(rows,key,[x[sourcekey] for x in tsv(D.parent/source)])
rules=tsv(D/'rule-crosswalk.tsv');assert len(rules)==211
for inv,source in [('phase-2i','phase-2i-certification.tsv'),('s7','s7-dispositions.tsv')]:
 exact([r for r in rules if r['inventory']==inv],'rule',[x['grammar-name'] for x in tsv(D.parent/source)])
methods=re.findall(r'^    pub fn (\w+)',(R/'src/runtime/src/runtime/program/compiler.rs').read_text(),re.M)
exact(tsv(D/'compiler-methods.tsv'),'method',methods);assert len(methods)==36
frontend=re.findall(r'^    pub fn (\w+)',(R/'src/engine/src/source_semantics/frontend.rs').read_text(),re.M)
exact(tsv(D/'frontend-apis.tsv'),'method',frontend)
catalog=[r['value']['name'] for r in observations if r['record']=='AUDIT_CATALOG']
exact(tsv(D/'catalog-overloads.tsv'),'export',catalog);assert len(catalog)==120
signatures=tsv(D/'catalog-signatures.tsv');assert len(signatures)==480
assert len({(x['export'],x['overload-id']) for x in signatures})==480
assert {x['export'] for x in signatures}==set(catalog)
for name,key,count in [('scalar-types.tsv','kind',17),('schema-families.tsv','schema',20)]:
 rows=tsv(D/name);assert len(rows)==count;assert len({x[key] for x in rows})==count
paths=tsv(D/'patch-ownership.tsv');assert len(paths)==91;assert len({x['path'] for x in paths})==91
print('Coverage accounting verified: 365 probes (317 pass observations, 48 failures), 24 gap groups, 27 consumers, 36 compiler methods, 24 frontend methods, 211 rule rows, 120 exports, 17 scalar kinds, 20 schema families, 91 patch paths. This is not an implementation seal.')
