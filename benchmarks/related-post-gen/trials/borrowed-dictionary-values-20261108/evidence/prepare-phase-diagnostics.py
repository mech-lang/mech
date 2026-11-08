from pathlib import Path
import difflib,hashlib,json,shutil,subprocess
P=Path(__file__).parent/'phase-diagnostics';P.mkdir(exist_ok=False);B=Path('/private/tmp/mech-related-native-dictionary-seed-frames-20261108/phase-diagnostics');S=Path('/private/tmp/mech-related-borrowed-dictionary-values-compiler-20261108');D=Path('/private/tmp/mech-related-ir306-native-string-profile-compiler-20261108');old=Path('/private/tmp/mech-related-native-dictionary-seed-frames-compiler-20261108');old_diag=Path('/private/tmp/mech-related-ir305-native-string-profile-compiler-20261108');assert not D.exists();shutil.copytree(S/'hosts/gpu',D/'hosts/gpu');patch=[];changes=[]
for f in (old_diag/'hosts/gpu').rglob('*'):
 if not f.is_file():continue
 rel=f.relative_to(old_diag);base=old/rel
 if not base.exists():shutil.copyfile(f,D/rel);changes.append(dict(path=str(rel),kind='diagnostic_only_added',sha256=hashlib.sha256(f.read_bytes()).hexdigest()));continue
 if base.read_bytes()==f.read_bytes():continue
 changes.append(dict(path=str(rel),kind='existing_diagnostic_hooks'))
 patch.extend(difflib.unified_diff(base.read_text().splitlines(True),f.read_text().splitlines(True),fromfile='a/'+str(rel),tofile='b/'+str(rel)))
f=P/'runtime-instrumentation.patch';f.write_text(''.join(patch));r=subprocess.run(['patch','-p1','--forward','-i',str(f)],cwd=D,capture_output=True,text=True);(P/'instrumentation-patch-application.json').write_text(json.dumps(dict(command=['patch','-p1','--forward','-i',str(f)],exit=r.returncode,stdout=r.stdout,stderr=r.stderr,changed=changes,production_Engine_unchanged=True,production_Objects_unchanged=True,diagnostic_only=True),indent=2)+'\n');assert r.returncode==0,r.stdout+r.stderr
for f in B.glob('*.py'):
 s=f.read_text().replace('/private/tmp/mech-related-native-dictionary-seed-frames-20261108','/private/tmp/mech-related-borrowed-dictionary-values-20261108').replace('/private/tmp/mech-related-native-dictionary-seed-frames-runners-20261108-v1','/private/tmp/mech-related-borrowed-dictionary-values-runners-20261108-v3').replace('/private/tmp/mech-related-native-dictionary-seed-frames-emission-control-20261108','/private/tmp/mech-related-borrowed-dictionary-values-emission-control-20261108-v3').replace('/private/tmp/mech-related-native-dictionary-seed-frames-opt-20261108-v4','/private/tmp/mech-related-borrowed-dictionary-values-opt-20261108-v5').replace('/private/tmp/mech-related-native-dictionary-seed-frames-compiler-20261108',str(S)).replace(str(old_diag),str(D)).replace('related_ir305_seed_dictionary_diagnostic_v1','related_ir306_borrowed_dictionary_values_diagnostic_v1').replace('32eb7450d54f32de12a70cefba0eda2f1f279be6','34afe038f933e3a2dff96a42b591f2617bd8fe2a').replace('99041021','99042021')
 (P/f.name).write_text(s)
print('IR306 diagnostic hooks prepared in separate runtime copy; no capture or build started')
