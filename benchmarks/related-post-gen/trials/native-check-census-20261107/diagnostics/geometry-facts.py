from pathlib import Path
import json,subprocess,time
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');O=Path('/private/tmp/mech-related-native-check-census-opt-20261107-v1')
def run(cmd,name):
 cmd=list(map(str,cmd));start=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True);(P/(name+'.json')).write_text(json.dumps(dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr,diagnostic_timings_excluded=True),indent=2)+'\n');assert r.returncode==0,r.stdout[-4000:]+r.stderr[-4000:];print(name,'passed',flush=True)
s=(P/'compiler-facts.rs').read_text();needle='    let identity = metadata["identity_symbol"].as_str().unwrap();';assert s.count(needle)==1
code='''    let mut geometry = Vec::new();
    for class in ["numeric_write_regions", "readonly_numeric_regions"] {
        for region in metadata[class].as_array().unwrap() {
            for (array, view) in region["proof"]["arrays"].as_array().unwrap().iter().enumerate() {
                let witness = view["witness"].as_u64().unwrap() as usize;
                let mech_engine::native_artifact::NativeIterationAction::LibraryCall { template, .. } = plan.blocks()[witness].action() else { panic!("geometry lost witness") };
                let schema = template.inputs()[0].body();
                let vector = matches!(schema, mech_core::SchemaBody::Matrix { dimensions, .. } if dimensions.len()==2 && dimensions.iter().any(|d|matches!(d,mech_core::DimensionExpr::Constant(1))));
                geometry.push(serde_json::json!({"class":class,"header":region["header"],"array":array,"witness":witness,"schema":schema,"static_vector":vector}));
            }
        }
    }
    std::fs::write(root.join("geometry.json"),serde_json::to_vec_pretty(&geometry).unwrap()).unwrap();
'''
s=s.replace(needle,code+needle);(P/'geometry-facts.rs').write_text(s)
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',P/'geometry-facts.rs','--out',P/'geometry-compiler'],'geometry-host-build')
run([P/'geometry-compiler',P/'build/source.mec',P/'geometry-Object'],'geometry-emission')
g=json.loads((P/'geometry-Object/geometry.json').read_text());q=[r for r in g if r['header']==1282];print('Query geometry static vectors:',sum(r['static_vector']for r in q),'/',len(q));print(json.dumps(q,indent=2)[:2000])
