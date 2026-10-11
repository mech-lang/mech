"""Finalize MATLAB audits and local chart data after the sequential suite finishes.

This process performs no benchmark computation and no publication. It waits for
the existing runner, then verifies the complete measured cohort before building
and checking the locally prepared site. Browser QA and supported Sites publication
remain explicit follow-up steps in the owning task.
"""
import argparse,hashlib,json,os,subprocess,sys,time
from datetime import datetime,timezone
from pathlib import Path

D=Path(__file__).resolve().parent
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def save(path,value):
    temporary=path.with_suffix(path.suffix+'.tmp');temporary.write_text(json.dumps(value,indent=2)+'\n');temporary.replace(path)
def alive(pid):
    result=subprocess.run(['ps','-p',str(pid),'-o','command='],capture_output=True,text=True)
    return result.returncode==0 and str(D/'run.py') in result.stdout and '--phase measure' in result.stdout

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--site',required=True,type=Path)
    parser.add_argument('--array-audit-dir',required=True,type=Path);parser.add_argument('--node',required=True,type=Path)
    args=parser.parse_args();site=args.site.resolve();previous=D/'evidence/chart-before-matlab.json'
    assert previous.exists()
    state_path=D/'results/completion.json';process=json.loads((D/'results/measurement-process.json').read_text())
    pinned_files=[site/name for name in ('build_data.py','augment_data.py','functional_data.py','matlab_data.py','verify.mjs','verify_previous.py',
                  'verify_ui.cjs','verify_years.cjs','verify_axes.cjs','language-years.json','dist/app.mjs','dist/math.mjs','dist/index.html')]
    state=dict(status='waiting-for-measurements',started_at=datetime.now(timezone.utc).isoformat(),measurement_pid=process['pid'],
               site=str(site),source_pins={str(p):sha(p) for p in pinned_files},completion_script_sha256=sha(Path(__file__)),
               previous_chart_sha256=sha(previous),steps=[])
    save(state_path,state);print('Waiting for the existing sequential MATLAB runner.',flush=True)
    try:
        while True:
            report=json.loads((D/'results/timings.json').read_text())
            if report['status']=='complete':break
            if not alive(process['pid']):raise RuntimeError('Measurement runner ended before the full suite completed. Preserve samples and inspect measurement.log before resuming.')
            time.sleep(30)
        for file,digest in state['source_pins'].items():assert sha(file)==digest,file
        state['status']='finalizing';save(state_path,state)
        environment=os.environ.copy()
        environment['PYTHONPATH']='/tmp/mech-swift-audit-deps:/tmp/mech-clbg-plot-libs'
        steps=[([sys.executable,str(D/'audit_loc_final.py')],D),([sys.executable,str(D/'summarize.py')],D),
               ([sys.executable,str(site/'build_data.py'),'--clbg-root',str(D.parents[1]),'--array-audit-dir',str(args.array_audit_dir)],site),
               ([str(args.node),str(site/'verify.mjs')],site),
               ([sys.executable,str(site/'verify_previous.py'),'--previous',str(previous),'--current',str(site/'dist/data.json'),
                 '--report',str(D/'results/previous-chart-preserved.json')],site)]
        for command,cwd in steps:
            print('STEP',Path(command[1]).name,flush=True)
            subprocess.run(command,cwd=cwd,env=environment,check=True)
            state['steps'].append(dict(script=Path(command[1]).name,status='passed'));save(state_path,state)
        state.update(status='ready-for-browser-qa-and-publication',finished_at=datetime.now(timezone.utc).isoformat(),
                     timing_report_sha256=sha(D/'results/timings.json'),summary_sha256=sha(D/'results/summary.json'),
                     chart_data_sha256=sha(site/'dist/data.json'),published=False)
        save(state_path,state);print('READY: full MATLAB cohort audited; local chart data checked. Browser QA and Sites publication pending.',flush=True)
    except BaseException as error:
        state.update(status='failed',error=str(error),failed_at=datetime.now(timezone.utc).isoformat());save(state_path,state);raise

if __name__=='__main__':main()
