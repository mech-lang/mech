from pathlib import Path
import copy,hashlib,json,shutil,subprocess,time
U=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
O=Path('/private/tmp/mech-related-deflate-exhaustion-proof-integration-opt-20261106-v1')
B=Path('/private/tmp/mech-related-native-reduction-terminal-transition-runners-20261106-v1')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text())
def run(c,p):
    c=list(map(str,c));t=time.monotonic();r=subprocess.run(c,capture_output=True,text=True)
    p.write_text(json.dumps(dict(command=c,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n')
    assert r.returncode==0,r.stdout+r.stderr
shutil.copyfile('/private/tmp/mech-related-native-numeric-collections-opt-20261106-v6/compiler-facts.rs',O/'compiler-facts.rs')
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',O/'compiler-facts.rs','--out',O/'compiler'],O/'compiler-build-run.json')
proof={}
for w in [4,8]:
    for label in ['after','compact']:
        p=U/f'{label}-{w}';p.mkdir(exist_ok=True)
        prior=B/f'{label}-{w}'
        if not (p/'compile.json').exists():
            run([O/'compiler',prior/'build/source.mec',p/'build'],p/'compile.json')
        assert read(p/'compile.json')['exit']==0
        files={f.name:sha(f)for f in (p/'build').iterdir()if f.is_file()}
        assert len(files)==13
        for file in ['source.mec','artifact.mecb','catalog.rs','catalog-profile.json','map-facts.txt','frame-summary.json']:
            assert files[file]==sha(prior/'build'/file),file
        before=read(prior/'build/metadata.json');after=read(p/'build/metadata.json')
        assert (after['typed_ir_version'],after['abi_version'],after['work_policy_version'],after['native_descriptor_version'])==(234,41,115,9)
        assert len(after['compiled_numeric_functions'])==53
        import struct
        def text_section(object_file):
            data=object_file.read_bytes()
            header=struct.unpack_from('<8I',data)
            assert header[0]==0xfeedfacf
            cursor=32;found=[]
            for _ in range(header[4]):
                command,size=struct.unpack_from('<II',data,cursor)
                if command==0x19:
                    nsects=struct.unpack_from('<I',data,cursor+64)[0]
                    for i in range(nsects):
                        at=cursor+72+80*i
                        name,segment,addr,length,offset=struct.unpack_from('<16s16sQQI',data,at)
                        if name.rstrip(b'\0')==b'__text':
                            found.append(data[offset:offset+length])
                            assert len(found[-1])==length
                cursor+=size
            assert len(found)==1
            return found[0]
        for kind,root in [('before',prior),('after',p)]:
            (p/(kind+'.native-text.bin')).write_bytes(text_section(root/'build/object.o'))
        same=(p/'before.native-text.bin').read_bytes()==(p/'after.native-text.bin').read_bytes()
        proof[f'{label}-{w}']=dict(source_artifact_catalog_frames_unchanged=True,numeric_functions=53,complete_native_text_byte_identical=same,before_native_text_sha256=sha(p/'before.native-text.bin'),after_native_text_sha256=sha(p/'after.native-text.bin'),before_code_bytes=before['code_bytes'],after_code_bytes=after['code_bytes'],files=files)
        print(label,w,'native text equal',same,'code',before['code_bytes'],after['code_bytes'],flush=True)
(U/'native-text-equivalence.json').write_text(json.dumps(proof,indent=2)+'\n')
assert all(x['complete_native_text_byte_identical']for x in proof.values())
