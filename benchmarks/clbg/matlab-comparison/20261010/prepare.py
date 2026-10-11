"""Prepare source/runtime/LOC pins for local MATLAB measurements."""
import hashlib,importlib.util,json,re,subprocess
from pathlib import Path
from datetime import datetime,timezone
from loc import audit
D=Path(__file__).resolve().parent;C=D.parents[1]
MATLAB=Path('/Applications/MATLAB_R2026b.app/bin/matlab')
BENCHES=['spectralnorm','mandelbrot','regexredux','revcomp','fasta','nbody','pidigits','fannkuchredux','knucleotide','binarytrees']
STDIN={'revcomp','regexredux','knucleotide'}
def sha(p):
    with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def main():
    executable=MATLAB.parent/'maca64/MATLAB';env={}
    homes=list((D/'toolchains').glob('*/Contents/Home'))
    if homes:env['MATLAB_JAVA']=str(homes[0])
    runtimes=dict(launcher=dict(path=str(MATLAB),sha256=sha(MATLAB)),engine=dict(path=str(executable),sha256=sha(executable)))
    stream_launcher=D/'matlab-stdin.sh'
    runtimes['input_routing']=dict(path=str(stream_launcher),sha256=sha(stream_launcher),role='runtime-support',policy='Only duplicates redirected stdin to descriptor 3 before MATLAB batch startup; no input parsing or benchmark computation.')
    if homes:
        p=homes[0]/'lib/server/libjvm.dylib';runtimes['java']=dict(path=str(p),sha256=sha(p),home=str(homes[0]))
        runtimes['java']['version']=subprocess.check_output([str(homes[0]/'bin/java'),'-version'],stderr=subprocess.STDOUT,text=True).strip()
        p=homes[0]/'lib/modules';runtimes['java_modules']=dict(path=str(p),sha256=sha(p))
    programs=[]
    for bench in BENCHES:
        for profile in ['compact','optimized']:
            source=D/'sources'/f'{bench}_{profile}.m';name=source.stem
            command=[str(MATLAB),'-licmode','onlinelicensing','-nodisplay','-sd',str(source.parent),'-batch',name+'('+('' if bench in STDIN else '{n}')+')']
            if bench in STDIN:command=['/bin/sh',str(stream_launcher)]+command
            programs.append(dict(key=bench+':matlab-'+profile,id=name,benchmark=bench,language='matlab',profile=profile,source=str(source),source_sha256=sha(source),
                source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/'+bench+'.html',source_provenance='Local MATLAB implementation of the official required algorithm; no published MATLAB timing or source ranking is claimed.',
                local_adaptation=True,command=command,runtime_environment=env,loc_audit=audit(source)))
    controls=json.loads((C/'functional-comparison/20261010/source-audit.json').read_text())['controls']
    for row in controls:assert sha(row['source'])==row['source_sha256'] and sha(row['binary'])==row['binary_sha256']
    built=dict(status='prepared',created_at=datetime.now(timezone.utc).isoformat(),programs=programs,controls=controls,runtimes=runtimes,
        runtime_policy='MATLAB R2026b 26.2.0.3386108 · native Apple Silicon · standard JIT enabled · eight automatic computation threads · base MATLAB only; no Parallel Computing Toolbox, Coder, or MEX benchmark executor.',
        licensing='Cached online licensing (-licmode onlinelicensing), headless fresh batch processes. User sign-in is not a timed operation.',
        timing_boundary='Fresh-process wall time includes launcher/licensing checks, MATLAB initialization, first-use JIT, complete input, computation, complete output to a regular file and teardown. Setup and validation excluded.',
        syntax_check='Pending MATLAB checkcode and small-output validation; prepare does not mark sources correct.')
    (D/'results/builds.json').write_text(json.dumps(built,indent=2)+'\n')
    print('Prepared',len(programs),'MATLAB sources and',len(controls),'unchanged Mech AOT controls')
if __name__=='__main__':main()
