from pathlib import Path
import json,collections,hashlib,tomllib,sys
root=Path(__file__).resolve().parents[4]; exp=Path(sys.argv[1]) if len(sys.argv)>1 else Path('/private/tmp/mech-v04-extraction'); c=json.loads((root/'audit/v04/data/census.json').read_text()); rows=c['files']

def summed(rs):
 return dict(files=len(rs),**{k:sum(r[k] for r in rs) for k in ('bytes','physical_lines','nonblank_lines')})
def groups(rs):
 return {role:summed([r for r in rs if r['role']==role]) for role in sorted(set(r['role'] for r in rs))}
def measured(rs,where):
 out=[]
 for r in rs:
  path=where(r['path']); b=path.read_bytes(); lines=b.decode('utf-8',errors='replace').splitlines() if r['physical_lines'] else []
  out.append(dict(r,bytes=len(b),physical_lines=(b.count(b'\n') + (1 if b and not b.endswith(b'\n') else 0)) if r['physical_lines'] else 0,nonblank_lines=sum(bool(s.strip()) for s in lines)))
 return out
rehomed = (exp/'mech/tests/stdlib-integration/Cargo.toml').exists()
def original_move(p):
 return p.startswith(('src/stdlib/','machines/'))
def moved(p):
 return original_move(p) and not (rehomed and (p.startswith('src/stdlib/tests/') or p == 'src/stdlib/build.rs'))
move_rows=[r for r in rows if original_move(r['path'])]; actual_move=[r for r in rows if moved(r['path'])]; estimated_move=[r for r in move_rows if not r['path'].startswith('src/stdlib/tests/') and r['path'] != 'src/stdlib/build.rs']; rest=[r for r in rows if not moved(r['path'])]
def external(p):
 return exp/'external'/('stdlib/'+p[len('src/stdlib/'):] if p.startswith('src/stdlib/') else p)
def retained_location(p):
 if rehomed and p.startswith('src/stdlib/tests/'):
  return exp/'mech/tests/stdlib-integration/tests'/p[len('src/stdlib/tests/'):]
 if rehomed and p == 'src/stdlib/build.rs':
  return exp/'mech/tests/stdlib-integration/build.rs'
 return exp/'mech'/p
measured_remaining=measured(rest,retained_location); measured_external=measured(actual_move,external)
if rehomed:
 p=exp/'mech/tests/stdlib-integration/Cargo.toml'; b=p.read_bytes()
 measured_remaining.append(dict(path='tests/stdlib-integration/Cargo.toml',role='configuration',bytes=len(b),physical_lines=b.count(b'\n'),nonblank_lines=sum(bool(s.strip()) for s in b.splitlines())))
 if (exp/'external/stdlib/build.rs').exists():
  b=(exp/'external/stdlib/build.rs').read_bytes()
  measured_external.append(dict(path='retained-inactive-build-script',role='build-tooling',bytes=len(b),physical_lines=b.count(b'\n'),nonblank_lines=sum(bool(s.strip()) for s in b.splitlines())))
related=[]
for r in rows:
 p=r['path']; action=None; reason=None
 if original_move(p):
  action='Separate' if p.startswith('src/stdlib/tests/') else 'Move'
  reason=('Distribution, type, memory and catalog integration contracts remain Mech-owned; relocate test targets and expose fixtures without reaching outside a package.' if action=='Separate' else 'Concrete standard operation implementation and private tests, or external distribution composition and its packaging.')
 elif p.startswith(('src/engine/src/resident/numeric/','src/engine/src/resident/general/')):
  action='Retain';reason='Resident arithmetic, value execution and artifact operation binding. Generated scalar-add links core/engine/runtime; direct external machine execution is separately verified through FunctionCatalog.'
 elif p.startswith('src/engine/src/intrinsics/'):
  action='Retain'; reason='Language operations for access, assignment, definition, conversion, structural construction and resident lowering; exposed through public intrinsic installers.'
 elif p=='src/core/src/stdlib.rs':
  action='Separate';reason='35 exported generic kernel/factory/lowering macros used by SDK consumers. Retain the exported SDK contract initially; separate family expansion helpers after external macro-hygiene and feature checks.'
 elif p.startswith('docs/stdlib/'):
  action='Separate';reason='Operation documentation and candidate-function list mix external functions with engine intrinsics; split ownership and repair stale engine/src/stdlib reference.'
 elif p in ['Cargo.toml','src/wasm/Cargo.toml','src/build/Cargo.toml','src/runtime/Cargo.toml','.github/ci/owners.toml','.github/workflows/ci.yml','.github/workflows/ci-full.yml'] or p.startswith(('src/build/src/dependency/','scripts/check-standard-machine','scripts/check-static-distribution','scripts/check-package-archives','scripts/check-native-linkage','scripts/check-source-catalog','scripts/check-function-system')):
  action='Separate';reason='Keep product integration/build ownership in Mech; replace local-machine path assumptions with external package/version inputs.'
 elif p.startswith(('tests/architecture/function-system/','tests/architecture/distributions/','tests/fixtures/bytecode-runtime-consumer/','tests/fixtures/bytecode-compiler-producer/','tests/fixtures/full-bytecode-runtime/','tests/fixtures/full-source-runtime/')) or p in ['docs/design/static-stdlib-composition.md','tests/catalog_closure.rs','tests/lazy_stdlib.rs','src/core/tests/stdlib_macro_hygiene.rs']:
  action='Retain';reason='Mech owns stable catalog identity, distribution integration, public SDK compatibility and complete product canaries.'
 if action:
  related.append({k:r[k] for k in ['path','blob','role','bytes','physical_lines','nonblank_lines']}|dict(disposition=action,reason=reason,experiment_disposition='Move' if moved(p) else 'Retain'))
manifest_records=[]
for r in rows:
 if original_move(r['path']) and r['path'].endswith('Cargo.toml'):
  parsed=tomllib.loads((root/r['path']).read_text()); sections={section:len(parsed.get(section,{})) for section in ['dependencies','dev-dependencies','build-dependencies']}; manifest_records.append(dict(path=r['path'],package=parsed['package']['name'],publish=parsed['package'].get('publish',True),features=len(parsed.get('features',{})),declared_dependency_entries=sections))
result=dict(metadata=dict(baseline=c['metadata']['baseline_commit'],units=c['metadata']['units'],scope='Baseline tracked files joined to census. Counts retain mixed production/test files as a separate role. Actual tree counts exclude generated Cargo.lock/build outputs and audit additions.',experiment='/private/tmp/mech-v04-extraction',old_paths_absent=not (exp/'mech/src/stdlib').exists() and not (exp/'mech/machines').exists()),arithmetic=dict(baseline=summed(rows),candidate_directory_relocation=summed(move_rows),estimated_moved=summed(estimated_move),estimated_retained_integration_tests=summed([r for r in move_rows if r['path'].startswith('src/stdlib/tests/')]),estimated_remaining_before_boundary_additions=summed([r for r in rows if r not in estimated_move]),actual_remaining=summed(measured_remaining),actual_external=summed(measured_external),actual_combined=summed(measured_remaining+measured_external),deleted_implementation=dict(files=0,physical_lines=0,bytes=0),added_rust_boundary_code=dict(files=0,physical_lines=0,bytes=0)),by_role=dict(baseline=groups(rows),candidate_move=groups(move_rows),estimated_move=groups(estimated_move),estimated_remaining=groups([r for r in rows if r not in estimated_move]),actual_remaining=groups(measured_remaining),actual_external=groups(measured_external),actual_combined=groups(measured_remaining+measured_external)),declared_manifests=manifest_records,records=related,limits=['The measured experiment verifies local package separation; permanent migration acceptance remains pending CI and release ownership checks.','Mech-owned integration tests are re-homed under tests/stdlib-integration; the JSON preserves the earlier coarse relocation stage.','A separate pre-existing source-only cfg defect was repaired: 8 lines removed and 2 added in engine define.rs. Relocation itself adds zero Rust implementation.','SDK registry publication and independently versioned release CI remain unverified; offline packaging fails on missing mech-engine registry metadata. The stdlib manifest has publish=false; registry release requires an explicit publishing policy change.'])
result['arithmetic']['separate_source_profile_repair'] = dict(removed_lines=8, added_lines=2, net_lines=-6)
result['arithmetic']['source_profile_feature_repair'] = dict(added_manifest_lines=10)
result['arithmetic']['added_test_harness_manifest'] = dict(files=1, physical_lines=52) if rehomed else dict(files=0,physical_lines=0)
result['metadata']['stage'] = 'rehomed-integration-tests-and-cfg-repair' if rehomed else 'coarse-relocation'
prior_path = root/'audit/v04/data/extraction-boundaries.json'
if prior_path.exists():
 prior=json.loads(prior_path.read_text())
 result['coarse_experiment_before_repairs']=prior.get('coarse_experiment_before_repairs', prior.get('arithmetic'))
(Path(sys.argv[2]) if len(sys.argv)>2 else root/'audit/v04/data/extraction-boundaries.json').write_text(json.dumps(result,indent=2)+'\n'); print(json.dumps(result['arithmetic'],indent=2))
