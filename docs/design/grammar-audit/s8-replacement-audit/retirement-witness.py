#!/usr/bin/env python3
"""Read-only evidence for G21/G22. Pass the frozen C checkout, not the B base."""
import argparse, json, pathlib, re
p=argparse.ArgumentParser()
p.add_argument('candidate',type=pathlib.Path)
p.add_argument('--require-closed',action='store_true')
a=p.parse_args()
checks={
 'G21': [
  ('src/runtime/src/runtime/program/compiler.rs',r'pub fn compile_tree\('),
  ('src/runtime/src/runtime/program/compiler.rs',r'plan_artifact_tree_with_services'),
  ('src/runtime/src/interactive.rs',r'pub fn from_tree\('),
 ],
 'G22': [
  ('src/wasm/src/project.rs',r'fn decode_document_tree\('),
  ('src/wasm/src/project.rs',r'tree: mech_core::nodes::Program'),
  ('src/wasm/src/project.rs',r'let _ = candidate_source;'),
 ]}
found=[]
for gap,checks in checks.items():
 for path,pattern in checks:
  text=(a.candidate/path).read_text()
  for m in re.finditer(pattern,text):
   found.append({'gap':gap,'path':path,'line':text.count('\n',0,m.start())+1,'evidence':m.group()})
print(json.dumps(found,indent=2))
if a.require_closed and found:
 raise SystemExit('retired compiler/browser ownership remains; inspection required')
