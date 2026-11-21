from pathlib import Path
from hashlib import sha256
import json,os,subprocess,time
W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');P=W/'benchmarks/mech-deflate/native-profile/native-compact-20261116';L=Path(__file__).resolve().parent;DEPS=L.parent/'target/release/deps';read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
source=(P/'evidence/batch-page-index/probes/parser.rs').read_text();old='#[path = "/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification/native-review-pr/checkout/src/compiler_allocator.rs"]';source=source.replace(old,'#[path = '+json.dumps(str(W/'src/compiler_allocator.rs'))+']')
source=source.replace('    let start=Instant::now();','    let repeats:usize=args.get(3).map(|x| x.parse().unwrap()).unwrap_or(1);\n    for iteration in 0..repeats {\n    let start=Instant::now();').replace('    std::fs::write(PathBuf::from(&args[2]),serde_json::to_vec_pretty(&report).unwrap()).unwrap();','    std::fs::write(PathBuf::from(&args[2]).with_extension(format!("{iteration}.json")),serde_json::to_vec_pretty(&report).unwrap()).unwrap();\n    }')
probe=L/'parser-probe.rs';probe.write_text(source);binary=L/'current-parser';libs={'mech_syntax':'d6ccf61cc22fcc4b','sha2':'58c79a6f6dca8f56','serde_json':'fcf6d49dea77c577','mimalloc':'14ebbbc9ab4f8cc1'};command=['rustc','+nightly-2026-03-03','--edition=2024','-C','opt-level=3','-C','codegen-units=16','-C','embed-bitcode=no','-D','warnings','-L','dependency='+str(DEPS),str(probe),'-o',str(binary)]
for name,h in libs.items():command+=['--extern',name+'='+str(DEPS/f'lib{name}-{h}.rlib')]
with (L/'probe-build.log').open('w') as log:subprocess.run(command,stdout=log,stderr=log,check=True)
receipt={'command':command,'probe_sha256':sha(probe),'binary_sha256':sha(binary),'dependencies':{name:{'path':str(DEPS/f'lib{name}-{h}.rlib'),'sha256':sha(DEPS/f'lib{name}-{h}.rlib')} for name,h in libs.items()},'parser_fingerprint':read(DEPS.parent/'.fingerprint/mech-syntax-d6ccf61cc22fcc4b/lib-mech_syntax.json'),'scope':'Current-source cached optimized parser library. Diagnostic standalone captures; no build-time claim. Fresh compiler source builds are required for promotion.'};(L/'probe-build.json').write_text(json.dumps(receipt,indent=2)+'\n')
source=P/'program.mec';captures=[]
for i in range(3):
 out=L/f'sample-{i}';out.mkdir();args=[str(binary),str(source),str(out/'parse.json'),'5'];env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}
 with (out/'stderr.log').open('w') as log:
  child=subprocess.Popen(args,env=env,stdout=log,stderr=log)
  sample=['/usr/bin/sample',str(child.pid),'1','1','-mayDie','-file',str(out/'sample.txt')]
  with (out/'sample.log').open('w') as stream:subprocess.run(sample,stdout=stream,stderr=stream,check=True)
  status=child.wait();assert status==0,status
 captures.append({'command':args,'sample_command':sample,'sample_sha256':sha(out/'sample.txt'),'observations':[read(p) for p in sorted(out.glob('parse.*.json'))]});print('Captured current parser',i,flush=True)
(L/'profile-captures.json').write_text(json.dumps(captures,indent=2)+'\n')
print('Current parser profile captures complete.',flush=True)
