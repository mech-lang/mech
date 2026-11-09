from pathlib import Path
import collections,hashlib,json,re,subprocess
P=Path(__file__).parent
roots={"default":Path("/private/tmp/mech-related-late-key-additions-runners-20261108-v1/histogram-reuse/build"),"control":Path("/private/tmp/mech-related-late-key-additions-control-20261108-v1/histogram-reuse/build")}
functions={};metadata={}
for label,root in roots.items():
 m=json.load(open(root/"metadata.json"));metadata[label]=m
 out=subprocess.check_output(["/usr/bin/otool","-tvV",str(root/"object.o")],text=True);(P/("native-"+label+".assembly.txt")).write_text(out)
 found={};name=None
 for line in out.splitlines():
  if line.endswith(":") and not line[0].isdigit():name=line[:-1];found[name]=[]
  elif name is not None and re.match(r"^[0-9a-f]{16}\t",line):found[name].append(line)
 functions[label]=found
changed=[]
for row in metadata["default"]["compiled_numeric_functions"]:
 header=row["header"];old=next(x for x in metadata["control"]["compiled_numeric_functions"]if x["header"]==header)
 if row["code_bytes"]==old["code_bytes"]and row["stack_bytes"]==old["stack_bytes"]:continue
 d={"header":header}
 for label,x in [("default",row),("control",old)]:
  lines=functions[label]["_"+x["symbol"]];ops=collections.Counter(line.split("\t")[1].split()[0]for line in lines)
  d[label]={"code_bytes":x["code_bytes"],"frame_bytes":x["stack_bytes"],"unsigned_guards_removed":x["unsigned_guards"],"static_instructions_including_alignment":len(lines),"opcode_counts":dict(ops),"static_stack_references":sum(bool(re.search(r"\[(?:sp|x29)(?:,|\])",line))for line in lines)}
 changed.append(d)
# All certificates are arithmetic-only; existing memory/ownership metadata is identical.
fields=["typed_loads","typed_stores","proven_projection_guards","proven_scalar_read_guards","proven_checked_increment_writes","proven_independent_view_increment_writes","owned_handoff_boundaries","first_change_boundaries","movable_capture_reads","ordinary_capture_reads","read_pack_sites","fully_lowered_read_packs","hoisted_pack_addresses"]
for field in ["compiled_numeric_write_loops","compiled_readonly_numeric_loops"]:
 assert len(metadata["default"][field])==len(metadata["control"][field])
 for a,b in zip(metadata["default"][field],metadata["control"][field]):
  assert a["header"]==b["header"]
  for key in fields:assert a[key]==b[key],(field,a["header"],key)
report={"changed_functions":changed,"default_code_bytes":metadata["default"]["code_bytes"],"control_code_bytes":metadata["control"]["code_bytes"],"array_and_ownership_metadata_unchanged":True,"capture_read_selection_unchanged":True,"tool":"/usr/bin/otool -tvV","limitation":"Static instruction/stack references include padding and are not dynamic spill costs or cache-pressure measurements."}
(P/"native-code-inspection.json").write_text(json.dumps(report,indent=2)+"\n")
print("Native code bytes",report["default_code_bytes"],report["control_code_bytes"])
for x in changed:
 print(x["header"],{label:{k:v for k,v in x[label].items()if k!="opcode_counts"}for label in ["default","control"]})
