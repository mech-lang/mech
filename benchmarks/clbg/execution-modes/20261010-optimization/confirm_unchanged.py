#!/usr/bin/env python3
from pathlib import Path
import importlib.util,json,random,statistics
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('confirmation_run',HERE/'run.py');run=importlib.util.module_from_spec(spec);spec.loader.exec_module(run)
old=json.loads((HERE.with_name('20261010')/'timings.json').read_text());new=json.loads((HERE/'timings.json').read_text())
keys=['knucleotide:mech-compact-final-std1-simd0-p4:jit','mandelbrot:mech-program-simd-loop-fma:resident']
report=dict(status='running',purpose='Paired confirmation of unchanged histogram and grid execution after small cohort timing shifts.',programs={})
for key in keys:
 rows={'before':old['programs'][key],'after':new['programs'][key]};samples={label:[] for label in rows};validations=[];rng=random.Random(key)
 for round_no in range(3):
  order=list(rows);rng.shuffle(order)
  for label in order:
   row=rows[label]
   assert run.sha(row['source'])==row['source_sha256'] and run.sha(row['binary'])==row['binary_sha256']
   while True:
    run.guard.wait_for_environment(run.POWER);result=run.execute(row,True,14400)
    if not result['competing_compilers'] and not result['power_changes']:break
   samples[label].append(result['seconds']);validations.append(dict(label=label,round=round_no+1,**result));print(key,label,round_no+1,result['seconds'],flush=True)
 medians={label:statistics.median(values) for label,values in samples.items()}
 report['programs'][key]=dict(samples=samples,medians=medians,after_over_before=medians['after']/medians['before'],runs=validations)
 (HERE/'confirmation.json').write_text(json.dumps(report,indent=2)+'\n')
report['status']='complete';(HERE/'confirmation.json').write_text(json.dumps(report,indent=2)+'\n')
