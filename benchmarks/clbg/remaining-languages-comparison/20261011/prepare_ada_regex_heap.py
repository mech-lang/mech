"""Repair generic Ada string storage; never mutate an already timed artifact."""
import argparse
import copy
import hashlib
import json
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from dotnet_fetch import C, D, sha
from metrics_final import normalize
from probe import save, validate_small

IDENT = 'regexredux-ada-pcre2-heap'
OLD_KEY = 'regexredux:regexredux-ada-pcre2'

def stage_source():
    original = D/'sources/ada/regexredux-ada-pcre2.adb'
    text = original.read_text()
    def replace(old, new):
        nonlocal text
        assert text.count(old) == 1, old
        text = text.replace(old, new)
    replace('with Interfaces.C;', 'with Ada.Strings.Unbounded.Aux;\nwith Interfaces.C;')
    replace('   package C renames Interfaces.C;', '   package C renames Interfaces.C;\n   package Aux renames Ada.Strings.Unbounded.Aux;')
    replace('   function Count(P,S : String) return Natural is',
            '   function Count(P : String; S : Unbounded_String) return Natural is\n'
            '      Subject : Aux.Big_String_Access;\n      Subject_Length : Natural;')
    replace('   begin\n      loop\n         RC := Match(Code,S\'Address,C.size_t(S\'Length),Offset,0,Data,Null_Address);',
            '   begin\n      Aux.Get_String(S,Subject,Subject_Length);\n      loop\n'
            '         RC := Match(Code,Subject.all\'Address,C.size_t(Subject_Length),Offset,0,Data,Null_Address);')
    start = text.index('   function Substitute(')
    end = text.index('   end Substitute;', start)+len('   end Substitute;')
    text = text[:start]+"""   function Substitute(P,R : String; S : Unbounded_String) return Unbounded_String is
      Code : constant Address := Pattern(P);
      Subject : Aux.Big_String_Access;
      Subject_Length : Natural;
      Needed : aliased C.size_t := 0;
      RC : C.int;
      Result : Unbounded_String;
      procedure Fill(Output : out String) is
      begin
         RC := Substitute_All(Code,Subject.all'Address,C.size_t(Subject_Length),0,16#100#,Null_Address,Null_Address,R'Address,C.size_t(R'Length),Output'Address,Needed'Access);
         if RC<0 then raise Constraint_Error; end if;
      end Fill;
   begin
      Aux.Get_String(S,Subject,Subject_Length);
      RC := Substitute_All(Code,Subject.all'Address,C.size_t(Subject_Length),0,16#1100#,Null_Address,Null_Address,R'Address,C.size_t(R'Length),Null_Address,Needed'Access);
      if RC /= -48 then raise Constraint_Error; end if;
      Aux.Set_String(Result,Positive(Needed),Fill'Access);
      Delete(Result,Integer(Needed)+1,Length(Result));
      Free_Code(Code);
      return Result;
   end Substitute;"""+text[end:]
    replace('   Text := To_Unbounded_String(Substitute(">.*\\n|\\n","",To_String(Text)));',
            '   Text := Substitute(">.*\\n|\\n","",Text);')
    replace('      Numbers.Put(Count(To_String(P),To_String(Text)),Width=>0);',
            '      Numbers.Put(Count(To_String(P),Text),Width=>0);')
    replace('      Text := To_Unbounded_String(Substitute(To_String(Substitutions(I)),To_String(Replacements(I)),To_String(Text)));',
            '      Text := Substitute(To_String(Substitutions(I)),To_String(Replacements(I)),Text);')
    text = text.replace('-- Full regex-redux in Ada; every application PCRE2/JIT FFI declaration counts.',
        '-- Full regex-redux; standard heap strings and borrowed views avoid stack copies.\n'
        '-- Every application PCRE2/JIT binding and buffer callback counts toward LOC.')
    source = D/f'sources/ada/{IDENT}.adb'
    source.write_text(text)
    return source

def reference(data):
    patterns = ('agggtaaa|tttaccct','[cgt]gggtaaa|tttaccc[acg]',
        'a[act]ggtaaa|tttacc[agt]t','ag[act]gtaaa|tttac[agt]ct',
        'agg[act]taaa|ttta[agt]cct','aggg[acg]aaa|ttt[cgt]ccct',
        'agggt[cgt]aa|tt[acg]accct','agggta[cgt]a|t[acg]taccct',
        'agggtaa[cgt]|[acg]ttaccct')
    clean = re.sub(b'>[^\n]*\n|\n', b'', data)
    out = ''.join(p+' '+str(len(re.findall(p.encode(),clean)))+'\n' for p in patterns).encode()
    sub = clean
    for pattern, replacement in ((b'tHa[Nt]',b'<4>'),(b'aND|caN|Ha[DS]|WaS',b'<3>'),
            (b'a[NSt]|BY',b'<2>'),(b'<[^>]*>',b'|'),(br'\|[^|][^|]*\|',b'-')):
        sub = re.sub(pattern,replacement,sub)
    return out+f'\n{len(data)}\n{len(clean)}\n{len(sub)}\n'.encode()

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--stage-only', action='store_true')
    args = ap.parse_args()
    source = stage_source()
    if args.stage_only:
        print(json.dumps(dict(staged=str(source), **normalize(source.read_text(),'ada'))))
        return
    native_path = D/'results/native-timings-v2.json'
    native = json.loads(native_path.read_text())
    assert native['status'] == 'complete' and not native.get('active'), 'Native timings must finish before compiling.'
    failure = native['programs'][OLD_KEY]['full_failure']
    assert 'stack overflow' in failure['error']
    parent_path = D/'source-audit-v2.json'
    parent = json.loads(parent_path.read_text())
    old = next(r for r in parent['programs'] if r['key'] == OLD_KEY)
    row = copy.deepcopy(old)
    for field in ('build_error','build_log','small_validation','compiled_source_files'):
        row.pop(field, None)
    build = D/f'support/ada-pascal/builds/{IDENT}'
    build.mkdir(parents=True,exist_ok=True)
    actual = build/'regexredux.adb'
    actual.write_bytes(source.read_bytes())
    binary = build/IDENT
    command = [str(actual) if arg in old['compiled_source_files'] else str(binary) if arg==old['binary'] else arg
        for arg in old['build_command']]
    result = subprocess.run(command,cwd=build,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=120)
    (build/'build.log').write_text(result.stdout)
    assert result.returncode == 0, result.stdout
    metric = normalize(source.read_text(),'ada')
    adaptation = ('Use standard GNAT Unbounded.Aux heap storage and borrowed string views instead of '
        'full-input To_String copies and stack-sized replacement buffers. Keep every count, PCRE2 call '
        'and ordered replacement; include the application allocation callback and all bindings in LOC.')
    row.update(id=IDENT,key='regexredux:'+IDENT,source=str(source),source_sha256=sha(source),
        binary=str(binary),binary_sha256=sha(binary),command=[str(binary)],build_command=command,
        compiled_source_files={str(actual):sha(actual)},build_status='passed',build_log=str(build/'build.log'),
        **metric,loc=metric['normalized_loc'],loc_expansions=metric['expansions'],application_source_files=[str(source)])
    row['adaptations'].append(adaptation)
    row['algorithm']['obligations'].append(adaptation)
    row['algorithm'].update(generic_heap_storage=True,borrowed_views=True)
    row['small_validation'] = validate_small(row)
    assert row['small_validation']['reference_match']
    dna = (b'agggtaaagggtaaaTTtttacccttttaccctcgggtaaaaaggtaaaagagtaaaaggataaa'
        b'agggcaaaagggtcaaagggtacaaagggtaagtHaNtHaTaNDcaNHaDHaSWaSaNaSaTBYS<hello>|abcd||Q|')
    fixtures = {
        'empty':b'', 'header-only':b'>THREE empty\n',
        'nonoverlap-and-ordered-substitutions':b'>THREE synthetic\n'+dna*17+b'\n',
        'line-read-boundaries':b'>THREE long\n'+b'A'*65533+dna+b'\n'+b'T'*65531+dna+b'\n',
        'heap-buffer-above-default-stack':b'>THREE large\n'+b'A'*9_000_000+dna+b'\n',
        'multiple-headers-and-lines':b'>ONE\n'+dna+b'\n>TWO\n'+dna.lower()+b'\n>THREE\n'+dna+b'\n',
    }
    probes = []
    for name,data in fixtures.items():
        expected = reference(data)
        process = subprocess.run(row['command'],input=data,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=120)
        assert process.returncode==0 and process.stdout==expected, (name,process.stderr,process.stdout,expected)
        probes.append(dict(name=name,passed=True,input_bytes=len(data),input_sha256=hashlib.sha256(data).hexdigest(),
            output_sha256=hashlib.sha256(process.stdout).hexdigest(),reference_sha256=hashlib.sha256(expected).hexdigest()))
    evidence = D/'results/ada-regex-repair-preflight.json'
    save(evidence,dict(status='passed',canonical=row['small_validation'],probes=probes,
        old_key=OLD_KEY,old_failure=failure,new_key=row['key'],changes=adaptation,
        source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256']))
    control = next(r for r in parent['controls'] if r['benchmark']=='regexredux')
    pins = {str(D/name):sha(D/name) for name in (
        'source-audit-v2.json','results/native-timings-v2.json','prepare_ada_regex_heap.py',
        'run_ada_regex_repair.py','queued_ada_regex_repair.py','run.py','probe.py','dotnet_fetch.py','metrics_final.py')}
    for path in (actual,evidence,C/'compact/run.py',C/'array-comparison/all-20261009/run.py',
            C/'regexredux/.build/input5000000.txt'):
        pins[str(path)] = sha(path)
    pins.update(parent['runtimes']['ada'].get('component_sha256',{}))
    libdir = Path(parent['runtimes']['ada']['prefix'])/'lib/gcc/aarch64-apple-darwin24.6.0/16.1.0/adainclude'
    for path in (libdir/'a-stunau.ads',libdir/'a-stunau.adb'):
        pins[str(path)] = sha(path)
    repair = dict(failed_key=OLD_KEY,replacement_key=row['key'],reason='Full-size stack overflow.',
        changes=adaptation,preflight=str(evidence),preflight_sha256=sha(evidence))
    audit = dict(status='reviewed',frozen_at=datetime.now(timezone.utc).isoformat(),programs=[row],
        controls=[control],runtimes={'ada':parent['runtimes']['ada']},
        libraries={k:v for k,v in parent['libraries'].items() if any(x in k for x in ('pcre2','gmp','ada-pascal/gnat'))},
        pin_files=pins,timing_boundary=parent['timing_boundary'],coverage={'ada':1},
        expected_benchmarks=['regexredux'],repairs=[repair],loc_policy=parent['loc_policy'],
        requirements=parent['requirements'],unmeasured_alternates=[])
    save(D/'ada-regex-repair-source-audit.json',audit)
    save(D/'ada-regex-repair-manifest.json',dict(programs=[row]))
    print(json.dumps(dict(status='reviewed',key=row['key'],loc=row['loc'],probes=len(probes))))

if __name__=='__main__':
    main()
