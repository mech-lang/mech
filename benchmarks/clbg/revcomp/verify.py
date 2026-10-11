#!/usr/bin/env python3
"""Independent byte, alias, table, record and error checks for native pathways."""
import itertools,json,os,random,subprocess,tempfile
from pathlib import Path
import run

def build(source,path,simd=True,fusion=1,reuse=1):
 src=path.with_suffix('.mec');src.write_text(source)
 env=os.environ|{'CARGO_TARGET_DIR':str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target'),'RUSTFLAGS':'-D warnings','MECH_BYTE_FUSION':str(fusion),'MECH_BYTE_REUSE':str(reuse)}
 argv=[str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'),'build',str(src),'--backend','cranelift','--offline','--no-config','-o',str(path)]
 if not simd:argv+=['--scalar']
 subprocess.run(argv,cwd=run.ROOT,env=env,check=True,stdout=subprocess.DEVNULL)
 return path

def execute(binary,data,partitions):
 return subprocess.run([str(binary)],input=data,capture_output=True,env=os.environ|{'MECH_SEQUENCE_PARTITIONS':str(partitions),'RAYON_NUM_THREADS':'4'},timeout=30)

def main():
 rng=random.Random(75281);checked=0;policies=[(False,0,0),(False,1,1),(True,0,1),(True,1,0),(True,1,1)]
 with tempfile.TemporaryDirectory(prefix='mech-byte-conformance-') as tmp:
  folder=Path(tmp)
  # Both compressed SIMD-eligible tables and arbitrary/dynamic scalar fallback.
  tables=[('ABCDabcd','zyxwzyxw','error',False),('0123456789','9876543210','error',False),('Az!','\x00zA','identity',False),('ABC','xyz','identity',True)]
  for ti,(keys,values,missing,dynamic) in enumerate(tables):
   from_expr=f'string/concat({json.dumps(keys[:1])},{json.dumps(keys[1:])})' if dynamic else json.dumps(keys)
   table=f'bytes/table({from_expr},{json.dumps(values).replace("\\u0000", "\\0")},{json.dumps(missing)})'
   for mode in ('translate','reverse','both','alias'):
    operation='bytes/translate(data,table)' if mode=='translate' else 'sequence/reverse(data)' if mode=='reverse' else 'bytes/translate(sequence/reverse(data),table)'
    suffix='io/write(text/wrap(data,7,"|"))\n' if mode=='alias' else ''
    source=f'data := bytes/from-text(io/read())\ntable := {table}\nio/write(text/wrap({operation},7,"|"))\n'+suffix
    mapping=dict(zip(keys.encode(),values.encode()))
    for pi,(simd,fusion,reuse) in enumerate(policies):
     binary=build(source,folder/f'table-{ti}-{mode}-{pi}',simd,fusion,reuse)
     for length in list(range(0,34))+[47,48,49,63,64,65,127,128,129,65535,65536,65537,131071,131072,131073]:
      alphabet=keys.encode() if missing=='error' and mode!='reverse' else bytes(range(256))
      data=bytes(rng.choice(alphabet) for _ in range(length));mapped=bytes(mapping.get(b,b) for b in data) if mode!='reverse' else data
      wanted=(mapped if mode=='translate' else mapped[::-1]);expected=b''.join(wanted[i:i+7]+b'|' for i in range(0,len(wanted),7))
      if mode=='alias':expected+=b''.join(data[i:i+7]+b'|' for i in range(0,len(data),7))
      for partitions in ([1,3,8] if length>=65535 else [1]):
       r=execute(binary,data,partitions);assert r.returncode==0 and r.stdout==expected,(ti,mode,pi,length,partitions,r.stderr,r.stdout[:80],expected[:80]);checked+=1
     if missing=='error' and mode!='reverse':
      for bad in range(256):
       if bad in mapping:continue
       r=execute(binary,bytes([keys.encode()[0]])*31+bytes([bad])+bytes([keys.encode()[-1]])*32,3)
       assert r.returncode!=0 and r.stdout==b'',(ti,mode,pi,bad,r.returncode,r.stdout);checked+=1
  # Generic record markers, joining separators, CRLF, final partial lines,
  # empty records, byte mapping, wrapping and repeatable stored-record aliases.
  for streaming in (False,True):
   source='''transform(header<string>, data<[u8]:_>) = (out<string>) :=
  table := bytes/table("ABCDabcd", "zyxwzyxw", "error")
  out := string/concat(string/concat(header,"\\n"),text/wrap(bytes/translate(sequence/reverse(data),table),5,"!")).
'''
   source+='records := '+('io/read-records("~", "")' if streaming else 'text/records(io/read(),"~","")')+'\nio/write-each(records/map(records,transform))\n'
   if not streaming:source+='io/write-each(records/map(records,transform))\n'
   for pi,(simd,fusion,reuse) in enumerate(policies):
    binary=build(source,folder/f'record-{streaming}-{pi}',simd,fusion,reuse)
    for input_data,expected in [(b'',b''),(b'~one\r\nABc\r\nD\r\n~two\n~three\nad',b'~one\nwxyz!~two\n~three\nwz!'),(b'~single',b'~single\n')]:
     if not streaming:expected*=2
     r=execute(binary,input_data,3);assert r.returncode==0 and r.stdout==expected,(streaming,pi,r.stderr,r.stdout,expected);checked+=1
    for data in (b'ABC\n~header\nAB\n',b'~header\nAx\n'):
     r=execute(binary,data,3);assert r.returncode!=0;checked+=1
  # Multi-byte line prefixes, marker bytes inside bodies, explicit joining,
  # dynamic width/separator fallback and the compiled serializer's vector tails.
  for width in (1,2,15,16,17,60,255,256,257):
   source=f"""copy(header<string>, data<[u8]:_>) = (out<string>) :=
  out := string/concat(string/concat(header,"\\n"),text/wrap(data,{width},"::")).
io/write-each(records/map(io/read-records("##", "|"),copy))
"""
   binary=build(source,folder/f'wrap-{width}')
   data=b'##first\r\nabc#def\r\n#partial\nzz\n##empty\n##last\n'+b'x'*517
   expected=b''
   for header,body in [(b'##first',b'abc#def|#partial|zz'),(b'##empty',b''),(b'##last',b'x'*517)]:
    expected+=header+b'\n'+b''.join(body[i:i+width]+b'::' for i in range(0,len(body),width))
   result=execute(binary,data,3);assert result.returncode==0 and result.stdout==expected,(width,result.stderr,result.stdout,expected);checked+=1
  # A pure source callback must stay typed and free from hidden IO/captures.
  bad_sources=[
   'f(a<u64>, b<[u8]:_>) = (out<string>) := out := "x".\nio/write-each(records/map(io/read-records(">",""),f))',
   'f(a<string>, b<[u8]:_>) = (out<u64>) := out := 1.\nio/write-each(records/map(io/read-records(">",""),f))',
   'f(a<string>, b<[u8]:_>) = (out<string>) := out := io/read().\nio/write-each(records/map(io/read-records(">",""),f))',
  ]
  for index,source in enumerate(bad_sources):
   path=folder/f'bad-{index}.mec';path.write_text(source)
   result=subprocess.run([str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'),'build',str(path),'--backend','cranelift','--offline','--no-config','-o',str(folder/'bad')],cwd=run.ROOT,capture_output=True)
   assert result.returncode!=0;checked+=1
  # Errors must be checked even when the compiler cannot compress a table.
  for name,expr in [('shape','bytes/table("ab","x","error")'),('duplicate','bytes/table("aa","xy","error")'),('policy','bytes/table("a","x","unknown")'),('width','bytes/table("a","x","error")')]:
   source=f'io/write(text/wrap(bytes/translate(bytes/from-text("a"),{expr}),{0 if name=="width" else 1},""))\n'
   binary=build(source,folder/name);r=execute(binary,b'',1);assert r.returncode!=0 and r.stdout==b'';checked+=1
 report={'all_passed':True,'checked_results':checked,'policies':policies,'all_byte_values_checked':True,'arbitrary_tables_and_dynamic_fallback':True,'independent_reference':True,'shared_input_preserved':True,'thread_partition_and_simd_tails':True,'generic_record_markers_and_empty_records':True,'io_errors_before_value_publication':True}
 (run.HERE/'results/byte-record-conformance.json').write_text(json.dumps(report,indent=2)+'\n');print(f'{checked} independent checks passed')
if __name__=='__main__':main()
