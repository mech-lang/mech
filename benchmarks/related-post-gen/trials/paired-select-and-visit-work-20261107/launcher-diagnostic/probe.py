from pathlib import Path
import sys,json,subprocess,time,random,resource,statistics
count=int(sys.argv[1]);p=Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json');text=p.read_text();held=[json.loads(text)for _ in range(count)];rows=[];rng=random.Random(990278020)
for sample in range(31):
 modes=['cwd','no_cwd'];rng.shuffle(modes)
 for mode in modes:
  start=time.monotonic_ns();r=subprocess.run(['/usr/bin/true'],cwd='/private/tmp'if mode=='cwd'else None,capture_output=True,text=True);seconds=(time.monotonic_ns()-start)/1e9;assert r.returncode==0 and not r.stdout and not r.stderr;rows.append(dict(sample=sample,mode=mode,seconds=seconds))
print(json.dumps(dict(held_outputs=count,held_items=sum(len(x)for x in held),maximum_resident_bytes=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,Python=sys.version,POSIX_SPAWN=subprocess._USE_POSIX_SPAWN,HAVE_CLOSEFROM=getattr(subprocess,'_HAVE_POSIX_SPAWN_CLOSEFROM',None),samples=rows,medians={mode:statistics.median(x['seconds']for x in rows if x['mode']==mode)for mode in modes},diagnostic_only=True,not_Rust_or_Mech_latency=True)))
