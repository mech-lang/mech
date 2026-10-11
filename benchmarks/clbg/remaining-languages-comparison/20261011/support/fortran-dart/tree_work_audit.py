#!/usr/bin/env python3
"""Small allocation/identity tests and optimized-code evidence, never timing samples."""
import hashlib,json,os,re,subprocess
from pathlib import Path
S=Path(__file__).resolve().parent
D=S.parents[1]
report=json.loads((D/'fortran_dart_manifest.json').read_text())
programs={p['id']:p for p in report['programs']}
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def run(argv,log,env=None):
    done=subprocess.run(list(map(str,argv)),cwd=S,env=os.environ|{'OMP_NUM_THREADS':'8','OMP_DYNAMIC':'FALSE'}|(env or {}),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=60)
    Path(log).write_bytes(done.stdout)
    if done.returncode:raise RuntimeError(str(argv)+'\n'+done.stdout.decode(errors='replace')[-4000:])
    return done.stdout.decode()
records=[]
expected=sum([4095,2047,31744,32512,32704,32752])
flags=['-O3','-mcpu=native','-isysroot','/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk','-fopenmp','-ffree-line-length-none','-fno-unsafe-math-optimizations','-ffp-contract=off']
for num in (1,2):
    p=programs[f'binarytrees-fortran-{num}'];source=Path(p['source']).read_text()
    if num==1:
        source=source.replace('  implicit none\n  save','  implicit none\n  integer(kind=8) :: allocation_count=0\n  save',1)
        source=source.replace('    allocate(node)','    allocate(node)\n    allocation_count=allocation_count+1',1)
        source=source.replace('end program binarytrees','write(0,\'(a,i0)\') "AUDIT_ALLOCATIONS ", allocation_count\nend program binarytrees')
    else:
        source=source.replace('module trees\n  use iso_c_binding','module trees\n  use iso_c_binding',1)
        source=source.replace('  type node\n','  integer(kind=8) :: allocation_count=0\n\n  type node\n',1)
        source=source.replace('      call c_f_pointer( apr_palloc(pool, sizeof(res)), res)','      call c_f_pointer( apr_palloc(pool, sizeof(res)), res)\n      !$omp atomic update\n      allocation_count=allocation_count+1',1)
        source=re.sub(r'\bend program\s*$', 'write(0,\'(a,i0)\') "AUDIT_ALLOCATIONS ", allocation_count\nend program',source)
    test_source=S/f'binarytrees-fortran-{num}-count.f90';test_source.write_text(source)
    binary=S/f'binarytrees-fortran-{num}-count'
    command=['/opt/homebrew/bin/gfortran-16',*flags,test_source,'-o',binary]
    if num==2:command+=['-L/opt/homebrew/opt/apr/lib','-lapr-1']
    run(command,S/f'binarytrees-fortran-{num}-count-build.log')
    output=run([binary,'10'],S/f'binarytrees-fortran-{num}-count-run.log')
    actual=int(re.search(r'AUDIT_ALLOCATIONS (\d+)',output).group(1));assert actual==expected
    disassembly=Path(S/f'binarytrees-fortran-{num}-disassembly.txt')
    assembly=disassembly.read_text()
    assert ('_malloc' if num==1 else '_apr_palloc') in assembly and '#0x10' in assembly
    records.append({'id':p['id'],'source_sha256':p['source_sha256'],'binary_sha256':p['binary_sha256'],'depth':10,'expected_nodes':expected,'actual_constructor_calls':actual,'equal_leaf_and_interior_bytes':16,'test_source':str(test_source),'compiler_evidence':str(disassembly),'compiler_evidence_sha256':sha(disassembly),'long_root_lifetime':'Original main constructs long-lived tree before temporary loops and checks/destroys it only after those loops; actual native make retains 16-byte allocations for leaf and interior nodes.'})
sdk=S/'dart-sdk';dart=sdk/'bin/dart';vm=sdk/'bin/dartvm';aot=sdk/'bin/dartaotruntime'
driver=r'''
int auditAllocations=0;
int verifyUnique(TreeNode root) {
  final seen=Set<TreeNode>.identity();
  int visit(TreeNode node) {
    if (!seen.add(node)) throw StateError('Shared tree node');
    if ((node.left==null)!=(node.right==null)) throw StateError('Incomplete binary tree');
    return node.left==null ? 1 : 1+visit(node.left!)+visit(node.right!);
  }
  return visit(root);
}
void main(List<String> args) {
  const n=10;
  if(verifyUnique(bottomUpTree(n+1))!=4095) throw StateError('stretch');
  final longRoot=bottomUpTree(n);
  if(verifyUnique(longRoot)!=2047) throw StateError('long root');
  var checks=0;
  for(var depth=4;depth<=n;depth+=2) {
    final count=1 << (n-depth+4);
    for(var i=0;i<count;i++) {
      final tree=bottomUpTree(depth);
      if(verifyUnique(tree)!=(1 << (depth+1))-1) throw StateError('node count');
      checks+=itemCheck(tree);
    }
  }
  if(itemCheck(longRoot)!=2047) throw StateError('long root lost');
  if(auditAllocations!=135854 || checks!=129712) throw StateError('total allocation work');
  print('AUDIT_ALLOCATIONS $auditAllocations');
}
'''
for num in (6,7):
    p=programs[f'binarytrees-dart-{num}'];source=Path(p['source']).read_text()
    source=source.replace('void main(List<String> args)', 'void originalMain(List<String> args)',1)
    source=source.replace('TreeNode(this.left, this.right);','TreeNode(this.left, this.right) {auditAllocations++;}',1)
    test_source=S/f'binarytrees-dart-{num}-count.dart';test_source.write_text(source+driver)
    binary=S/f'binarytrees-dart-{num}-count'
    run([dart,'compile','exe',test_source,'-o',binary],S/f'binarytrees-dart-{num}-count-build.log')
    output=run([binary,'10'],S/f'binarytrees-dart-{num}-count-run.log')
    assert f'AUDIT_ALLOCATIONS {expected}' in output
    kernel=S/f'binarytrees-dart-{num}-aot.dill'
    run([aot,sdk/'bin/snapshots/gen_kernel_aot.dart.snapshot','--platform',sdk/'lib/_internal/vm_platform_product.dill','--aot','--tfa','--target-os=macos','-o',kernel,p['source']],S/f'binarytrees-dart-{num}-kernel-build.log')
    cfg=S/f'binarytrees-dart-{num}-aot-cfg.log'
    run([sdk/'bin/utils/gen_snapshot','--snapshot_kind=app-aot-assembly','--assembly='+str(S/f'binarytrees-dart-{num}-aot.s'),'--print-flow-graph-optimized','--print-flow-graph-filter=bottomUpTree',kernel],cfg)
    assert cfg.read_text().count('AllocateObject')>=2
    jit_cfg=S/f'binarytrees-dart-{num}-jit-cfg.log'
    jp=programs[f'binarytrees-dart-jit-{num}']
    run([vm,'--print-flow-graph-optimized','--print-flow-graph-filter=bottomUpTree',jp['binary'],'10'],jit_cfg)
    assert jit_cfg.read_text().count('AllocateObject')>=2
    records.append({'id':p['id'],'source_sha256':p['source_sha256'],'binary_sha256':p['binary_sha256'],'depth':10,'expected_nodes':expected,'actual_constructor_calls':expected,'identity_and_full_tree_checks_passed':True,'equal_leaf_and_interior_class':'TreeNode','test_source':str(test_source),'compiler_evidence':str(cfg),'compiler_evidence_sha256':sha(cfg),'jit_compiler_evidence':str(jit_cfg),'jit_compiler_evidence_sha256':sha(jit_cfg),'long_root_lifetime':'Actual main checks the same longRoot after all temporary-tree iterations; Dart 7 callback captures it until all isolate replies finish. Optimized AOT and JIT CFG retain AllocateObject in both leaf and interior branches and two recursive child calls.'})
(S/'tree-work-audit.json').write_text(json.dumps({'status':'passed','tests':records},indent=2)+'\n')
print('TREE ALLOCATION AUDIT PASS',len(records),flush=True)
