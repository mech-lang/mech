from pathlib import Path
import difflib,json,subprocess,shutil,time

R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
U=Path('/private/tmp/mech-related-selected-ranges-borrowed-json-runners-20261107-v1');U.mkdir()
versions={242:(Path('/private/tmp/mech-related-conjoined-window-integration-opt-20261107-v1'),Path('/private/tmp/mech-related-conjoined-window-integration-runners-20261107-v1/reuse-4')),
          252:(Path('/private/tmp/mech-related-selected-range-integration-opt-20261107-v1'),Path('/private/tmp/mech-related-selected-range-integration-runners-20261107-v1/reuse-4'))}
def run(c,p):
 c=list(map(str,c));start=time.monotonic();x=subprocess.run(c,capture_output=True,text=True);p.write_text(json.dumps(dict(command=c,exit=x.returncode,seconds=time.monotonic()-start,stdout=x.stdout,stderr=x.stderr),indent=2)+'\n');assert x.returncode==0,x.stderr
for epoch,(O,B)in versions.items():
 p=U/str(epoch);p.mkdir();(p/'build').symlink_to(B/'build',target_is_directory=True)
 original=(B/'runner.rs').read_text();text=original
 replacements=[
  ('struct Post {',"struct Post<'input> {"),
  ('#[serde(rename = "_id")]\n    id: String,','#[serde(borrow, rename = "_id")]\n    id: std::borrow::Cow<\'input, str>,'),
  ('    title: String,','    #[serde(borrow)]\n    title: std::borrow::Cow<\'input, str>,'),
  ("posts: &'a [Post]","posts: &'a [Post<'_>]"),
  ('posts: &mut [Post]','posts: &mut [Post<\'_>]'),
  ('let mut posts: Vec<Post> = serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();',
   'let json_input = std::fs::read_to_string(&args[1]).unwrap();\n    let mut posts: Vec<Post<\'_>> = serde_json::from_str(&json_input).unwrap();'),
 ]
 for old,new in replacements:
  assert text.count(old)==1,(epoch,old);text=text.replace(old,new)
 (p/'runner.rs').write_text(text)
 (p/'host-source.patch').write_text(''.join(difflib.unified_diff(original.splitlines(True),text.splitlines(True),fromfile='owned-json.rs',tofile='borrowed-json.rs')))
 reverse=text
 for old,new in reversed(replacements):reverse=reverse.replace(new,old)
 assert reverse==original
 (p/'scope.json').write_text(json.dumps(dict(compiler_epoch=epoch,native_program_bundle_unchanged=True,source_reverse_patch_exact=True,change='Borrow ID/title JSON text with Cow and owned fallback for escapes. Tags retain owned transfer into canonical inputs. All parsing stays in the same JSON window; all input, setup and teardown remain in whole-process time. This is host IO only, no algorithm or compiler bypass.'),indent=2)+'\n')
 run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',B/'build','--linked'],p/'host-build-run.json')
 run(['python3',R/'audit-native-link.py','--object',B/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
 run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',B/'build/source.mec','--out',p/'edges'],p/'edge-run.json')
 print(epoch,'borrowed JSON host audited and31 Rust edge outputs exact',flush=True)
