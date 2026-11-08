from pathlib import Path
import difflib,hashlib,json,shutil,subprocess,time
P=Path(__file__).parent/'phase-diagnostics';P.mkdir(exist_ok=False)
B=Path('/private/tmp/mech-related-generic-native-dictionary-20261108/phase-diagnostics')
source=Path('/private/tmp/mech-related-native-dictionary-seed-frames-compiler-20261108')
diag=Path('/private/tmp/mech-related-ir305-native-string-profile-compiler-20261108');assert not diag.exists()
old=Path('/private/tmp/mech-related-generic-native-dictionary-compiler-20261108');old_diag=Path('/private/tmp/mech-related-ir304-native-string-profile-compiler-20261108')
shutil.copytree(source/'hosts/gpu',diag/'hosts/gpu')
changes=[];patch=[]
for f in (old_diag/'hosts/gpu').rglob('*'):
 if not f.is_file():continue
 rel=f.relative_to(old_diag);base=old/rel
 if not base.exists():shutil.copyfile(f,diag/rel);changes.append(dict(path=str(rel),kind='diagnostic_only_added',sha256=hashlib.sha256(f.read_bytes()).hexdigest()));continue
 if f.read_bytes()==base.read_bytes():continue
 changes.append(dict(path=str(rel),kind='existing_diagnostic_hooks'))
 patch.extend(difflib.unified_diff(base.read_text().splitlines(True),f.read_text().splitlines(True),fromfile='a/'+str(rel),tofile='b/'+str(rel)))
f=P/'runtime-instrumentation.patch';f.write_text(''.join(patch));r=subprocess.run(['patch','-p1','--forward','-i',str(f)],cwd=diag,capture_output=True,text=True);(P/'instrumentation-patch-application.json').write_text(json.dumps(dict(command=['patch','-p1','--forward','-i',str(f)],exit=r.returncode,stdout=r.stdout,stderr=r.stderr,changed=changes,production_Engine_unchanged=True,production_Objects_unchanged=True,diagnostic_only=True),indent=2)+'\n');assert r.returncode==0,r.stdout+r.stderr
for f in B.glob('*.py'):
 s=f.read_text().replace('/private/tmp/mech-related-generic-native-dictionary-20261108','/private/tmp/mech-related-native-dictionary-seed-frames-20261108').replace('/private/tmp/mech-related-generic-native-dictionary-runners-20261108-v1','/private/tmp/mech-related-native-dictionary-seed-frames-runners-20261108-v1').replace('/private/tmp/mech-related-generic-native-dictionary-emission-control-20261108','/private/tmp/mech-related-native-dictionary-seed-frames-emission-control-20261108').replace('/private/tmp/mech-related-generic-native-dictionary-opt-20261108-v6','/private/tmp/mech-related-native-dictionary-seed-frames-opt-20261108-v4').replace('/private/tmp/mech-related-generic-native-dictionary-compiler-20261108','/private/tmp/mech-related-native-dictionary-seed-frames-compiler-20261108').replace('/private/tmp/mech-related-ir304-native-string-profile-compiler-20261108',str(diag)).replace('related_ir304_dictionary_diagnostic_v1','related_ir305_seed_dictionary_diagnostic_v1').replace('ebaac8e7944d24bbb6c1938a3514cde86511711d','32eb7450d54f32de12a70cefba0eda2f1f279be6').replace('99040021','99041021')
 (P/f.name).write_text(s)
print('Five diagnostic hook changes applied to separate IR305 runtime copy; production Engine and Objects unchanged')
