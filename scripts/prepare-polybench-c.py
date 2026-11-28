#!/usr/bin/env python3
"""Build the original F64 MINI C adapters from a pinned PolyBench/C 4.2.1 checkout.

This preserves the first-baseline adapter generator, including its timing and
cache boundaries. No kernel or initializer is rewritten. See the fixture README.
"""
from pathlib import Path
import argparse, hashlib, json, math, re, subprocess, time
ROOT=Path(__file__).resolve().parents[1]
SUITE=None
def clean(text): return re.sub(r'/\*.*?\*/','',text,flags=re.S)
def loops(array):
    name,dims=array
    order=list(range(len(dims)))
    prefix=''.join(f'for (int p{x}=0;p{x}<({dims[x]});++p{x})' for x in order)
    cell=f'(POLYBENCH_ARRAY({name}))'+''.join(f'[p{x}]' for x in range(len(dims)))
    return prefix,cell
def wrapper(path):
    source=clean(path.read_text()); main=source[source.index('int main'):]; prefix=main[main.index('{')+1:main.index('polybench_start_instruments;')]
    declarations,init=prefix.split('init_array',1)
    init='init_array'+init
    arrays=[];array_types={}
    for rank,arguments in re.findall(r'POLYBENCH_(\d)D_ARRAY_DECL\((.*?)\);',declarations,re.S):
        parts=[p.strip() for p in arguments.split(',')]; arrays.append((parts[0],parts[2+int(rank):]));array_types[parts[0]]=parts[1]
    integers=re.findall(r'int\s+(\w+)\s*=\s*([^;]+);',declarations)
    scalars=re.findall(r'DATA_TYPE\s+(\w+)\s*;',declarations)
    kernel=re.search(r'kernel_\w+\s*\(.*?\);',main,re.S).group()
    outputs=re.search(r'polybench_prevent_dce\((.*?)\);',main,re.S).group(1)
    output_names=re.findall(r'POLYBENCH_ARRAY\((\w+)\)',outputs)
    if not output_names: output_names=['out']
    head=f'''#define main polybench_original_main
#include "{path}"
#undef main
#include "{SUITE/'utilities/polybench.c'}"
#include <time.h>
#include <stdlib.h>
#if defined(__APPLE__)
#define MECH_POLYBENCH_CLOCK CLOCK_UPTIME_RAW
#else
#define MECH_POLYBENCH_CLOCK CLOCK_MONOTONIC
#endif
static void number(double x) {{ if (isnan(x)) printf("\\\"NaN\\\""); else if (isinf(x)) printf("\\\"%sInfinity\\\"", x<0?"-":""); else printf("%.17g",x); }}
static double now(void) {{ struct timespec t; clock_gettime(MECH_POLYBENCH_CLOCK, &t); return t.tv_sec+t.tv_nsec*1e-9; }}
int main(int argc,char **argv) {{
'''+declarations
    # Defined scratch contents make snapshots deterministic. Initialized C inputs overwrite these.
    head+='\n'.join(f'memset({name},0,sizeof({array_types[name]})*({"*".join(f"({d})" for d in dims)}));' for name,dims in arrays)
    head+='\n'+init
    head+='\nif(argc>1 && strcmp(argv[1],"inputs")==0){printf("{"); int sep=0;\n'
    for name,_ in integers:
        head+=f'if(sep++)printf(",");printf("\\\"{name}\\\":{{\\\"u64\\\":%llu}}",(unsigned long long){name});\n'
    for name in scalars:
        # Output-only scalars must not be read before the kernel.
        if name=='out': continue
        head+=f'if(sep++)printf(",");printf("\\\"{name}\\\":");number({name});\n'
    for array in arrays:
        name,dims=array; rows='*'.join(f'({d})' for d in dims[:-1]) if len(dims)>1 else dims[0];cols=dims[-1] if len(dims)>1 else '1'
        p,cell=loops(array)
        head+=f'if(sep++)printf(",");printf("\\\"{name.strip("_")}\\\":{{\\\"shape\\\":[%d,%d],\\\"values\\\":[",{rows},{cols});int sep_{name}=0;{p}{{if(sep_{name}++)printf(",");number({cell});}}printf("]}}");\n'
    head+='printf("}\\n");return 0;}\n'
    for name,dims in arrays:
        size=f'sizeof({array_types[name]})*('+ '*'.join(f'({d})' for d in dims)+')'
        head+=f'size_t bytes_{name}={size};void *saved_{name}=malloc(bytes_{name});memcpy(saved_{name},{name},bytes_{name});\n'
    head+=kernel+'\n'
    head+='int fresh=argc>2 && strcmp(argv[2],"fresh")==0; int samples=argc>1?atoi(argv[1]):0; if(samples<1)samples=1; double *times=malloc(sizeof(double)*samples);\nfor(int run=0;run<samples;++run){\n'
    for name,_ in arrays: head+=f'memcpy({name},saved_{name},bytes_{name});\n'
    head+='\n'
    for name,_ in arrays:head+=f'if(fresh)free({name});\n'
    head+='polybench_flush_cache();double start=now();\n'
    for name,dims in arrays:
        head+=f'if(fresh){{{name}=(__typeof__({name}))polybench_alloc_data(bytes_{name},1);memcpy({name},saved_{name},bytes_{name});}}\n'
    head+=kernel+'\ntimes[run]=now()-start;\n}\nprintf("{\\\"values\\\":[");int sep=0;\n'
    for name in output_names:
        array=next((a for a in arrays if a[0]==name),None)
        if array:
            p,cell=loops(array);head+=f'{p}{{if(sep++)printf(",");number({cell});}}\n'
        else:head+=f'if(sep++)printf(",");number({name});\n'
    head+='printf("],\\\"samples_seconds\\\":[");for(int i=0;i<samples;++i){if(i)printf(",");printf("%.17g",times[i]);}printf("]}\\n");return 0;}\n'
    return head,{'arrays':dict(arrays),'array_types':array_types,'outputs':output_names,'source':str(path.relative_to(SUITE)),'source_sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
def main():
    global SUITE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--suite', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--compiler', default='clang')
    parser.add_argument('--kernel')
    args = parser.parse_args()
    SUITE = args.suite.resolve()
    fixture = json.loads((ROOT/'hosts/gpu/tests/fixtures/polybench/cases.json').read_text())
    args.output.mkdir(parents=True, exist_ok=True)
    records = {}
    for case in fixture['cases']:
        name = case['name']
        if args.kernel and name != args.kernel:
            continue
        path = SUITE/case['upstream_c_path']
        if hashlib.sha256(path.read_bytes()).hexdigest() != case['upstream_c_sha256']:
            raise ValueError(f'upstream source changed: {name}')
        text, record = wrapper(path)
        c = args.output/f'{name}.c'
        c.write_text(text)
        binary = args.output/name
        cmd = [args.compiler, *fixture['c_flags'], '-I', str(SUITE/'utilities'),
               '-I', str(path.parent), str(c), '-lm', '-o', str(binary)]
        subprocess.run(cmd, check=True, capture_output=True, text=True)
        raw_inputs = subprocess.check_output([str(binary), 'inputs'], text=True)
        inputs = {k.strip('_').replace('_','-'): v for k,v in json.loads(raw_inputs).items()}
        if inputs != case['inputs']:
            raise ValueError(f'C initializer differs from fixture: {name}')
        values = json.loads(subprocess.check_output([str(binary), '1'], text=True))['values']
        if len(values) != len(case['expected']) or not all(
            isinstance(a, (int, float)) and math.isfinite(a)
            and math.isclose(a, b, rel_tol=1e-10, abs_tol=1e-9)
            for a,b in zip(values, case['expected'])
        ):
            raise ValueError(f'C kernel differs from fixture: {name}')
        (args.output/f'{name}.inputs.json').write_text(raw_inputs)
        record.update(compile_command=cmd, binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                      validation_count=len(values), status='ok')
        records[name] = record
        print(name, 'ok', flush=True)
    if not records:
        raise ValueError('no matching kernels')
    (args.output/'manifest.json').write_text(json.dumps({
        'suite': fixture['suite'], 'dataset': fixture['dataset'],
        'numeric_profile': fixture['numeric_profile'],
        'upstream_archive': fixture['upstream_archive'], 'upstream_sha256': fixture['upstream_sha256'],
        'compiler': subprocess.check_output([args.compiler, '--version'], text=True).strip(),
        'cases': records,
    }, indent=2)+'\n')

if __name__ == '__main__':
    main()
