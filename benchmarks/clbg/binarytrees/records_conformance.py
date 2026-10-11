#!/usr/bin/env python3
"""Exercise general records, payload links, scalar codecs and layout proofs."""
import json,os,subprocess
import conformance as c
import run

def passed(name,source,expected):
 binary=c.build(name,source)
 actual=c.output(binary)
 assert actual==expected,(name,actual,expected)
 c.checks.append([name,'byte-exact'])

def main():
 mixed='''<entry> := <{value<u64>, factor<f64>, enabled<bool>, next<ref>}>
Echo(row<entry>) = (result<entry>) :=
  result := row.
Test(value<u64>, factor<f64>, enabled<bool>) = (result<u64>) :=
  tail := memory/allocate({value: 7, factor: factor, enabled: enabled, next: memory/null()})
  head := memory/allocate({value: value, factor: 2.5, enabled: false, next: tail})
  first := Echo(memory/read(head, <entry>))
  last := memory/read(first.next, <entry>)
  result := control/select(first.value == 29 && first.factor == 2.5 && last.factor == 1.25 && last.enabled && last.next == memory/null(), first.value + last.value, 0).
result := memory/with-arena(2, <entry>, Test, (29, 1.25, true))
io/print("{}\\n", result)
'''
 passed('linked-records-with-payloads',mixed,b'36\n')
 # Same source with out-of-line aggregate arguments/results and scalar codecs.
 previous=c.ENV.get('MECH_CONTROL_INLINE');c.ENV['MECH_CONTROL_INLINE']='0'
 try:passed('out-of-line-record-codecs',mixed,b'36\n')
 finally:
  if previous is None:c.ENV.pop('MECH_CONTROL_INLINE')
  else:c.ENV['MECH_CONTROL_INLINE']=previous
 plain='''<accum> := <{i<u64>, total<u64>}>
More(row<accum>) = (more<bool>) :=
  more := row.i < 10.
Step(row<accum>) = (result<accum>) :=
  result := {i: row.i + 1, total: row.total + row.i}.
row := control/while({i: 0, total: 0}, More, Step)
io/print("{} {}\\n", row.i, row.total)
'''
 passed('ordinary-record-loop-state',plain,b'10 45\n')
 nested='''<position> := <{x<f64>, y<f64>}>
<row> := <{pos<position>, count<u64>, active<bool>}>
Test(n<u64>) = (result<u64>) :=
  h := memory/allocate({pos: {x: 1.25, y: 2.5}, count: 19, active: true})
  row := memory/read(h, <row>)
  result := control/select(row.pos.x == 1.25 && row.pos.y == 2.5 && row.active, row.count, 0).
result := memory/with-arena(1, <row>, Test, 0)
io/print("{}\\n", result)
'''
 passed('nested-ordinary-records',nested,b'19\n')
 floating='''<row> := <{value<f64>}>
Test(n<f64>) = (result<f64>) :=
  h := memory/allocate({value: n})
  row := memory/read(h, <row>)
  result := row.value.
r := memory/with-arena(1, <row>, Test, 1.25)
io/print("{}\\n", control/select(r == 1.25, 1, 0))
'''
 passed('float-scope-result',floating,b'1\n')
 boolean=floating.replace('f64','bool').replace('1.25','true')
 passed('boolean-scope-result',boolean,b'1\n')
 mismatch='''<row> := <{x<u64>, y<u64>}>
<wrong> := <{x<f64>, y<f64>}>
Test(n<u64>) = (result<u64>) :=
  h := memory/allocate({x: 1, y: 2})
  row := memory/read(h, <wrong>)
  result := 1.
r := memory/with-arena(1, <row>, Test, 0)
io/print("{}\\n", r)
'''
 c.fail('same-arity-wrong-field-kinds',mismatch,9)
 c.fail('same-arity-wrong-field-names',mismatch.replace('<{x<f64>, y<f64>}>','<{a<u64>, b<u64>}>'),9)
 c.reject('unknown-record-field',plain.replace('row.total)','row.missing)'))
 c.reject('duplicate-record-fields','r := {x: 1, x: 2}\nio/print("{}\\n", r.x)\n')
 c.reject('field-annotation-mismatch','r := {x<u64>: 1.25}\nio/print("{}\\n", 1)\n')
 copied='''<row> := <{value<u64>, factor<f64>, enabled<bool>}>
Test(row<row>) = (result<row>) :=
  h := memory/allocate(row)
  result := memory/read(h, <row>).
r := memory/with-arena(1, <row>, Test, {value: 23, factor: 1.25, enabled: true})
io/print("{}\\n", control/select(r.factor == 1.25 && r.enabled, r.value, 0))
'''
 passed('scalar-record-scope-arguments-and-results',copied,b'23\n')
 c.reject('variable-annotation-mismatch','r<{x<u64>}> := {x: 1.25}\nio/print("{}\\n", 1)\n')
 c.reject('references-hidden-in-record-results','''<row> := <{link<ref>}>
Test(n<u64>) = (result<row>) :=
  result := {link: memory/null()}.
r := memory/with-arena(1, <row>, Test, 0)
io/print("{}\\n", 1)
''')
 c.reject('recursive-kind-alias-cycle','<row> := <{child<row>}>\nr := io/print("{}\\n", 1)\n')
 conditional='''<row> := <{value<u64>}>
Yes(n<u64>) = (result<u64>) :=
  h := memory/allocate({value: n})
  result := n.
No(n<u64>) = (result<u64>) :=
  result := n.
r := control/if(false, Yes, No, 17)
io/print("{}\\n", r)
'''
 passed('unused-heap-branch-without-arena',conditional,b'17\n')
 c.fail('branch-proof-cannot-escape',conditional+'h := memory/allocate({value: 19})\nio/print("{}\\n", 1)\n',9)
 empty='''More(i<u64>, end<u64>) = (more<bool>) :=
  more := i < end.
Step(i<u64>, end<u64>) = (next<u64>, stop<u64>) :=
  h := memory/allocate({value: i})
  next := i + 1
  stop := end.
(i, stop) := control/while((0, 0), More, Step)
io/print("{}\\n", i)
'''
 passed('empty-heap-loop-without-arena',empty,b'0\n')
 c.fail('loop-proof-cannot-escape',empty+'h := memory/allocate({value: 19})\nio/print("{}\\n", 1)\n',9)
 ordering='''<row> := <{value<u64>}>
Capacity(n<u64>) = (result<u64>) :=
  printed := io/print("capacity={}\\n", n)
  result := 1.
Argument(n<u64>) = (result<u64>) :=
  printed := io/print("argument={}\\n", n)
  result := n.
Body(n<u64>) = (result<u64>) :=
  h := memory/allocate({value: n})
  result := n.
r := memory/with-arena(Capacity(0), <row>, Body, Argument(7))
io/print("result={}\\n", r)
'''
 passed('memory-scope-argument-order',ordering,b'capacity=0\nargument=7\nresult=7\n')
 before_guard='''Argument(n<u64>) = (result<u64>) :=
  printed := io/print("argument={}\\n", n)
  result := n.
r := memory/allocate({value: Argument(7)})
io/print("{}\\n", 1)
'''
 binary=c.build('memory-argument-effects-before-guard',before_guard)
 result=subprocess.run([str(binary)],env=c.ENV,capture_output=True)
 assert result.returncode and b'native control error 9' in result.stderr and result.stdout==b'argument=7\n',(result.stdout,result.stderr)
 c.checks.append(['memory-argument-effects-before-guard','source-order-preserved'])
 report={'compiler_sha256':run.sha(c.MECH),'passed':len(c.checks),'checks':c.checks}
 (run.HERE/'results/records-conformance.json').write_text(json.dumps(report,indent=2)+'\n')
 print('Passed',len(c.checks),'general record checks',flush=True)
if __name__=='__main__':main()
