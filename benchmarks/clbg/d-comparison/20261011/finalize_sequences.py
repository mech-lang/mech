from pathlib import Path
import subprocess,hashlib,json,sys
D=Path(__file__).resolve().parent;C=D.parents[1]
sys.path.insert(0,str(C/'remaining-languages-comparison/20261011'))
import probe
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
rows=[]
for f in sorted((D/'sources').glob('*/*.d')):
 bench=f.parent.name
 if bench not in ('fasta','knucleotide','revcomp','regexredux'):continue
 name=bench+'-d-'+f.stem;b=D/'binaries'/name
 command=['/opt/homebrew/bin/ldc2','-O3','-release','-boundscheck=off','-mcpu=native',str(f),'-of='+str(b)]
 result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 log=D/'results'/(name+'.build.log');log.write_bytes(result.stdout)
 assert result.returncode==0,result.stdout.decode()
 if bench=='fasta':
  orig=D/'upstream/sequences/hanabi1224-bench-algorithm-fasta-1.d'
  adap=['Restore canonical binary64 seed/139968 direct division and unscaled cumulative probabilities; no period cache or integer thresholds.']
  if f.stem=='compact':adap+=['Locally authored compact form of the published D algorithm using std.algorithm probability search and range-generated ALU lines.']
  elif f.stem=='optimized':adap+=['Locally authored buffered form: native D byte buffer and statically known probability width; every PRNG step and binary64 division/comparison executed.']
 elif bench=='knucleotide':
  orig=D/('upstream/sequences/kragen-bench-knucleotide-knucleotide.dlang' if f.stem=='compact' else 'upstream/sequences/hanabi1224-bench-algorithm-knucleotide-1-m.d')
  adap=['Read canonical stdin and select >THREE by header; accept arbitrary record line lengths, upper/lowercase; stop at subsequent header.','Construct all seven complete growing frequency maps for k=1,2,3,4,6,12,18; never count only requested keys.']
  if f.stem=='compact':adap+=['Update D1 high-level map source to D2 std.algorithm, immutable string keys and exact modern writefln formatting.','Use lexical ascending tie break, as required by the existing complete-output reference.', 'Return empty complete maps when sequence is shorter than k.']
  else:adap+=['Remove Nullable wrapper from packed keys because modern Phobos Nullable no longer satisfies hash-key signature; use the same Code payload directly.', 'Sort equal-frequency results by decoded lexical nucleotide order and return empty maps for sequence shorter than k; tested tied and short inputs.','Replace file-name and fixed61-byte inputchunk assumptions with standard stdin FASTA parsing; retain all seven rolling maps and five independent worker tasks.']
 elif bench=='revcomp':
  orig=D/('upstream/sequences/kragen-bench-revcomp-revcomp.dlang-3.dlang' if f.stem=='compact' else 'upstream/sequences/kragen-bench-revcomp-revcomp.dlang-4.dlang')
  adap=['Update D1 source to D2 standard sequence/range APIs and preserve full uppercase IUB complement mapping and60-column output.']
  if f.stem=='optimized':adap+=['Locally authored buffered adaptation: read complete stdin with byChunk, lookup each reversed byte and rewrap each complete record in native D source.']
 else:
  orig=D/('upstream/sequences/hanabi1224-bench-algorithm-regex-redux-1.d' if f.stem=='compact' else 'upstream/sequences/hanabi1224-bench-algorithm-regex-redux-'+f.stem.rsplit('-',1)[1]+'.d')
  adap=['Replace file-name input with canonical stdin byChunk/join.','Run actual generic std.regex header/newline removal, all nine nonoverlapping pattern searches and all five sequential replacements. No hand-coded benchmarkmatcher.', 'All searches and replacements are case-sensitive, with default std.regex flags matching the canonical current regex-redux reference (no i flag); matchAll supplies nonoverlapping matches and replaceAll handles every replacement.']
  if f.stem=='compact':adap+=['Locally authored compact equivalent using runtime regex compilation, with all pattern strings and replacements counted.']
  else:adap+=['Use walkLength instead of materializing match ranges as an array; every match is still found by the same std.regex engine.','Add explicit valid module name for hyphenated filename.']
 prov=json.loads(Path(str(orig)+'.provenance.json').read_text())
 row=dict(id=name,key=bench+':'+name,benchmark=bench,language='d',source=str(f),source_sha256=sha(f),binary=str(b),binary_sha256=sha(b),command=[str(b)]+(['{n}'] if bench=='fasta' else []),build_command=command,build_status='passed',build_log=str(log),source_url=prov['source_url'],upstream_source=str(orig),upstream_sha256=sha(orig),upstream_revision=prov['revision'],adaptations=adap,execution_mode='aot',runtime_environment={},algorithm=dict(strict=True,native_benchmark_kernel=False,reason='Complete canonical workload in D application source; ordinary D standard-library algorithms only.',arithmetic='binary64' if bench=='fasta' else 'integer'),loc_review='Root applies shared comment-free normalization including all statements and initialized bindings packed on one line.')
 row['small_validation']=probe.validate_small(row)
 rows.append(row)
 print(name,'SMALL PASS',flush=True)
(D/'sequence-manifest.json').write_text(json.dumps(dict(status='prepared',programs=rows,compiler_version=subprocess.check_output(['/opt/homebrew/bin/ldc2','--version'],text=True),provenance_notes='Historical D Benchmark Game entries are from the preserved kragen/shootout archive. Modern sequence sources are published by Hanabi; compact adaptations and optimized buffered programs are explicitly locally authored.'),indent=2)+'\n')
