"""Sample program counters inside complete native calls in an instrumented worker."""
from collections import Counter
from bisect import bisect_right
from hashlib import file_digest
from importlib import import_module
from pathlib import Path
import argparse
import gzip
import json
import subprocess
from pinned_worker_rebuild import rebuild_link

q=Path(__file__).resolve().parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('label')
parser.add_argument('cases', nargs='+')
parser.add_argument('--llvm-object', type=Path)
parser.add_argument('--samples', type=int, default=2)
parser.add_argument('--intervals', type=int, nargs=2, default=(509, 773))

args = parser.parse_args()
assert 2 <= args.samples <= 8192 and all(20 <= x <= 10000 for x in args.intervals)
label = args.label
folder = q / label
cases = args.cases
assert len(cases) == 1
record_label = f"{label}-{cases[0]}"
assert len(cases) == len(set(cases))
original=q/'pinned-host-link'/'mech-worker.rs'
c_object=folder/'worker-pc-probe.o'
subprocess.run(['cc','-std=gnu11','-O2','-Wall','-Wextra','-Werror','-c',
                q/'worker-pc-probe.c','-o',c_object],check=True)

def sha(path):
    with path.open('rb') as stream:
        return file_digest(stream,'sha256').hexdigest()

source=original.read_text()
def replace(before,after):
    global source
    assert source.count(before)==1,before
    source=source.replace(before,after)
replace('unsafe extern "C" {', '''unsafe extern "C" {
    fn mech_probe_pc_start(microseconds: u32) -> core::ffi::c_int;
    fn mech_probe_pc_scope(active: core::ffi::c_int);
    fn mech_probe_pc_stop() -> core::ffi::c_int;
    fn mech_probe_pc_read(index: u32) -> usize;
    fn mech_probe_pc_image(pc: usize, anchor: usize, image: *mut *const core::ffi::c_char,
        symbol: *mut *const core::ffi::c_char) -> core::ffi::c_int;''')
replace('    println!("CASE_READY\\t{}", case.name);','''    let interval = std::env::var("MECH_PC_SAMPLE_INTERVAL").unwrap().parse().unwrap();
    assert_eq!(unsafe { mech_probe_pc_start(interval) }, 0);
    println!("CASE_READY\\t{}", case.name);''')
replace('        let start = Instant::now();','''        unsafe { mech_probe_pc_scope(1) };
        let start = Instant::now();''')
replace('        let seconds = start.elapsed().as_secs_f64();','''        let seconds = start.elapsed().as_secs_f64();
        unsafe { mech_probe_pc_scope(0) };''')
replace('    drop((input, prefix, expected, current, arguments));','''    let count = unsafe { mech_probe_pc_stop() };
    assert!((1..32768).contains(&count));
    eprintln!("PC_BASE {} {} {}", case.name, mech_compact_closed_v2 as *const () as usize, count);
    let anchor = mech_compact_closed_v2 as *const () as usize;
    let mut unique = std::collections::BTreeSet::new();
    for index in 0..count as u32 {
        let pc = unsafe { mech_probe_pc_read(index) };
        eprintln!("PC_SAMPLE {} {pc}",case.name);
        unique.insert(pc);
    }
    for pc in unique {
        let mut image = core::ptr::null();
        let mut symbol = core::ptr::null();
        if unsafe { mech_probe_pc_image(pc, anchor, &mut image, &mut symbol) } == 0 || image.is_null() {
            continue;
        }
        let image = unsafe { std::ffi::CStr::from_ptr(image) }.to_string_lossy();
        let symbol = if symbol.is_null() {
            "".into()
        } else {
            unsafe { std::ffi::CStr::from_ptr(symbol) }.to_string_lossy()
        };
        eprintln!("PC_IMAGE {} {pc} {}", case.name, serde_json::json!([image, symbol]));
    }
    drop((input, prefix, expected, current, arguments));''')
instrumented=original.with_name('worker-pc-probe.rs')
if instrumented.is_file():
    assert instrumented.read_text()==source
else:
    instrumented.write_text(source)

worker=folder/'pc-profile-worker-resolved'
runtime = json.loads((folder / 'runtime.json').read_text())
recorded = runtime['link']
assert recorded.count(str(original)) == recorded.count(str(folder/'mech-worker')) == 1
profile_link = [str(instrumented) if arg == str(original) else str(worker)
                if arg == str(folder/'mech-worker') else arg for arg in recorded]
argv, link_provenance = rebuild_link(profile_link)
assert [arg for arg in argv if arg.startswith('mech_gpu=')] == [
    arg for arg in recorded if arg.startswith('mech_gpu=')]
subprocess.run(argv+['-C',f'link-arg={c_object}'],check=True)
fixture=q/'fixtures.json'
indices={row['name']:i for i,row in enumerate(json.loads(fixture.read_text()))}
command=[str(worker),str(folder/'metadata.json'),str(fixture)]

def run(interval):
    import os
    payload=''.join(f'C\t{indices[c]}\t{args.samples - 2}\n' + 'W'*4 + 'N'*args.samples for c in cases)+'Q\n'
    result=subprocess.run(command,input=payload,text=True,capture_output=True,timeout=45,
                          env=dict(os.environ,MECH_PC_SAMPLE_INTERVAL=str(interval)))
    stdout=q/f'{record_label}-pcs-resolved-{interval}.stdout.gz'
    stderr=q/f'{record_label}-pcs-resolved-{interval}.stderr.gz'
    stdout.write_bytes(gzip.compress(result.stdout.encode(), compresslevel=4, mtime=0))
    stderr.write_bytes(gzip.compress(result.stderr.encode(), compresslevel=4, mtime=0))
    result.check_returncode()
    measured=[json.loads(row) for row in result.stdout.splitlines() if row.startswith('{')]
    assert len(measured)==args.samples*len(cases) and {row['case'] for row in measured}==set(cases)
    assert all(row['exact_reference'] and row['independent_inflation'] and row['input_alias_unchanged']
               and row['borrowed_output_alias_unchanged'] for row in measured)
    assert 'CLEANUP private_payload_bytes=0 live_owner_records=0' in result.stderr
    return result.stderr

m=import_module('reachability-census')
_,sections,symbols=m.load(worker)
symbols=m.text_symbols(sections,symbols)
anchor=symbols['_mech_compact_closed_v2']
_,text_address,text_size,*_=sections[m.TEXT]
order=sorted((address,name) for name,address in symbols.items())
starts=[address for address,_ in order]
llvm_symbols = []
llvm_text_size = 0
if args.llvm_object:
    metadata = json.loads((folder/'metadata.json').read_text())['checked_root_machine']
    assert metadata['llvm_object_sha256'] == sha(args.llvm_object)
    _, llvm_sections, definitions = m.load(args.llvm_object)
    start = m.text_symbols(llvm_sections, definitions)[m.ROOT]
    assert start == 0
    llvm_text_size = llvm_sections[m.TEXT][2]
    llvm_symbols = sorted((at, name) for name, at in m.text_symbols(llvm_sections, definitions).items()
                          if name.startswith('_') and (at or name == m.ROOT))
    assert llvm_symbols[0] == (0, m.ROOT)
llvm_addresses = [at for at, _ in llvm_symbols]
counters={name:Counter() for name in cases}
placements={}
images={}
for interval in args.intervals:
    for line in run(interval).splitlines():
        tag, *fields = line.split()
        if tag=='PC_BASE':
            case,origin,count=fields
            placements[case]=int(origin),int(count)
        elif tag=='PC_IMAGE':
            _,case,pc,names=line.split(maxsplit=3)
            address=int(pc)-placements[case][0]+anchor
            image,symbol=json.loads(names)
            images[case,address]=Path(image).name+(' '+symbol if symbol else '')
        elif tag=='PC_SAMPLE':
            case,pc=fields
            address=int(pc)-placements[case][0]+anchor
            counters[case][address]+=1
results={}
for case,counter in counters.items():
    per_symbol=Counter()
    per_offset={}
    for pc,count in sorted(counter.items()):
        if not text_address<=pc<text_address+text_size:
            per_symbol[images.get((case,pc),'outside executable text')]+=count
            continue
        index=bisect_right(starts,pc)-1
        assert index>=0
        start,symbol=order[index]
        if symbol == m.ROOT and 0 <= pc - start < llvm_text_size:
            at = bisect_right(llvm_addresses, pc - start) - 1
            relative, symbol = llvm_symbols[at]
            start += relative
        per_symbol[symbol]+=count
        per_offset.setdefault(symbol,{})[str(pc-start)]=count
    results[case]=dict(total=counter.total(),symbols=per_symbol,offsets=per_offset)
    print(case,counter.total(),'samples',per_symbol.most_common(7),flush=True)
record=dict(artifact=label,profile_worker_sha256=sha(worker),object_sha256=sha(folder/'program.o'),
            metadata_sha256=sha(folder/'metadata.json'),link=argv,
            timed_calls_per_case=args.samples, intervals_us=args.intervals,
            llvm_object_sha256=sha(args.llvm_object) if args.llvm_object else None,
            link_provenance=link_provenance,cases=results)
(q/f'{record_label}-pcs-resolved.json').write_text(json.dumps(record,indent=2)+'\n')
