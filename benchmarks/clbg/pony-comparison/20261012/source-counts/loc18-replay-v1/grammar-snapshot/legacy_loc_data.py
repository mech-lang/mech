"""Optional reviewed legacy LOC overlay; intentionally not hooked into builder.

This importer cannot admit a parser proposal. It requires a reviewed complete
pool audit and source-specific decisions before changing derived chart metadata.
Measured source bytes, samples, reports and frozen experiment inputs stay intact.
Call only while holding the shared timing mutex (hashing/source packaging occurs).
"""
import copy
import fcntl
import hashlib
import json
import math
import os
import statistics
from pathlib import Path

COHORT = "loc-normalization/20261012"


def validate_reviewed_audit(audit):
    if audit.get("status") != "reviewed":
        raise ValueError("Legacy LOC parser proposals require explicit review before chart admission.")
    if not audit.get("review", {}).get("complete_measured_pool"):
        raise ValueError("The review must cover the complete measured candidate pool.")
    if not audit.get("coverage_complete"):
        raise ValueError("A completed study still lacks full candidate-pool reconciliation.")
    if audit.get("pending_needed_measurement"):
        raise ValueError("A prospective compact winner still needs qualification/full measurements.")
    reviewed = audit["review"].get("source_decisions", {})
    required_sources={identity for row in audit["candidates"] for identity in row["source_identities"]}
    for row in audit.get("current_source_reconciliation",[]):required_sources.update(row["source_identities"])
    for row in [*audit.get("paired_control_source_reconciliation",[]),
                *audit.get('fresh_control_source_reconciliation',[])]:required_sources.update(row["source_identities"])
    for row in [*audit.get("current_source_reconciliation",[]),*audit.get("paired_control_source_reconciliation",[]),
                *audit.get('fresh_control_source_reconciliation',[])]:
        if not row["source_identities"] or row.get("canonical_parser_loc") is None:
            raise ValueError("An endpoint/control lacks verified source bytes for LOC review.")
    if audit.get("displayed_comparisons",{}).get("unresolved"):
        raise ValueError("A displayed paired LOC denominator lacks an exact historical source identity.")
    for identity in required_sources:
        source=audit["sources"][identity]
        decision = reviewed.get(identity)
        if not decision or decision.get("source_sha256") != source["sha256"]:
            raise ValueError("Missing source-keyed review: " + identity)
        if not isinstance(decision.get("normalized_loc"), int) or decision["normalized_loc"] <= 0:
            raise ValueError("Invalid reviewed LOC: " + identity)
    for row in audit["candidates"]:
        if row.get("normalized_loc") is None or len(row["samples"]) < 3:
            raise ValueError("Candidate lacks a source count/full measurement set: " + row["key"])
        if row["seconds"] != statistics.median(row["samples"]):
            raise ValueError("Measured samples/median changed: " + row["key"])
        if not row.get("qualification",{}).get("admitted"):
            raise ValueError("Candidate lacks reviewed full-workload qualification: " + row["key"])
        qualification=row["qualification"]
        if not (qualification.get("full_workload") and qualification.get("full_output_validated")
                and qualification.get("report_completed") and qualification.get("sample_count_matches")
                and qualification.get("sample_count")==len(row["samples"])):
            raise ValueError("Candidate full-record qualification does not match its sample vector: "+row["key"])
        if not all(type(s) in (int,float) and math.isfinite(s) and s>0 for s in row["samples"]):
            raise ValueError("Invalid measured samples: " + row["key"])
    return reviewed


def apply_legacy_loc_audit(root, data, audit_path, *, common_timing_mutex_fd=None):
    """Apply only a reviewed metadata correction; caller owns timing boundary."""
    if common_timing_mutex_fd is None:
        raise ValueError("LOC packaging/hashing requires the held common timing mutex.")
    expected=os.stat("/tmp/mech-clbg-timing.lock")
    actual=os.fstat(common_timing_mutex_fd)
    if (actual.st_dev,actual.st_ino)!=(expected.st_dev,expected.st_ino):
        raise ValueError("The supplied descriptor is not the common timing mutex.")
    try:
        fcntl.flock(common_timing_mutex_fd,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:
        raise ValueError("Timing is active; this descriptor does not own the mutex.") from None
    root, audit_path = Path(root), Path(audit_path)
    audit = json.loads(audit_path.read_text())
    decisions = validate_reviewed_audit(audit)
    sha = lambda path:hashlib.sha256(Path(path).read_bytes()).hexdigest()
    if sha(audit["current_data"]["path"])!=audit["current_data"]["sha256"]:
        raise ValueError("Explorer snapshot changed after LOC review; explicitly reconcile the new revision.")
    expected_projection=audit["current_legacy_projection"]
    auxiliary_keys={row["key"] for row in audit.get("current_source_reconciliation",[])}
    actual_projection={key:{field:row.get(field) for field in ("language","benchmark","loc","seconds","samples","files")}
        for key,row in data["programs"].items() if row["language"] in {"c","cpp","rust","go","java","chapel","julia","lua","luajit","python","ruby","perl","php","javascript","mech","mech-jit","mech-resident"} or key in auxiliary_keys}
    if actual_projection!=expected_projection:
        raise ValueError("Passed legacy dataset differs from the reviewed input projection.")
    exclusions={row["key"] for row in data.get("exclusions",[]) if row.get("key")}
    if any(row["key"] in exclusions for row in audit["candidates"]):
        raise ValueError("The current required-work review excludes a LOC candidate.")
    for relative, expected in audit["report_pins"].items():
        if sha(root/relative) != expected:
            raise ValueError("Measured report changed after LOC review: " + relative)
    result = copy.deepcopy(data)
    candidates = {}
    changed = []
    for row in audit["candidates"]:
        count = sum(decisions[identity]["normalized_loc"] for identity in row["source_identities"])
        # Apply canonical source decisions directly, never add a delta to a
        # producer's already-normalized count. Preserve that older policy value.
        previous = result["programs"].get(row["key"], {})
        if previous and (previous["samples"] != row["samples"] or previous["seconds"] != row["seconds"]):
            raise ValueError("Current timing differs from reviewed pool: " + row["key"])
        file_ids = []
        for ref in row["source_files"]:
            path = Path(ref["path"])
            if sha(path) != ref["sha256"]:
                raise ValueError("Measured source bytes changed: " + str(path))
            identity = ref["sha256"]
            file_ids.append(identity)
            if identity not in result["files"]:
                try:
                    public = str(path.relative_to(root))
                except ValueError:
                    public = str(path)
                result["files"][identity] = dict(name=path.name,path=public,
                    sha256=identity,code=path.read_bytes().decode("utf-8"))
        program = dict(previous, key=row["key"], id=row["id"],
            benchmark=row["benchmark"], language=row["language"],
            loc=count, original_loc=row["original_loc"], physical_loc=row["physical_loc"],
            producer_normalized_loc=row.get("authoritative_loc"),
            loc_metric="reviewed-canonical-source-units", loc_audit=COHORT,
            seconds=row["seconds"], samples=row["samples"], timing_sd=statistics.stdev(row["samples"]),
            report=row["report"],source_report=row["source_report"],files=file_ids,
            measurement_status="measured",algorithm=row.get("algorithm"))
        for field in ("runtime_policy","execution_mode","runtime","command","cwd","runtime_environment", "source_url",
                      "qualification","full_validation","sample_validation","full_output_validation", "required_work_validated",
                      "timing_boundary","protocol","required_work_evidence","original_physical_loc"):
            if field in row:program[field]=row[field]
        if row.get('fresh_paired_mech_control') is not None:
            raw_control=row['fresh_paired_mech_control']
            identities=row.get('fresh_paired_mech_control_source_identities',[])
            if not identities:
                raise ValueError('Fresh paired control lacks exact reviewed source ownership: '+row['key'])
            control_path=Path(raw_control['source'])
            control_sha=raw_control['source_sha256']
            if identities != [control_sha+':mech']:
                raise ValueError('Fresh paired control count is bound to another source identity: '+row['key'])
            if sha(control_path)!=control_sha:
                raise ValueError('Fresh paired control source bytes changed: '+str(control_path))
            if control_sha not in result['files']:
                try:public=str(control_path.relative_to(root))
                except ValueError:public=str(control_path)
                result['files'][control_sha]=dict(name=control_path.name,path=public,sha256=control_sha,
                    code=control_path.read_bytes().decode('utf-8'))
            # Keep the raw producer LOC and samples; attach a separately named
            # reviewed value rather than relabelling the old timing control.
            control=copy.deepcopy(raw_control)
            control.update(canonical_loc=sum(decisions[i]['normalized_loc'] for i in identities),
                canonical_source_identities=list(identities),files=[control_sha],
                loc_metric='reviewed-canonical-source-units',loc_audit=COHORT)
            program['fresh_paired_mech_control']=control
        candidates.setdefault((row["benchmark"],row["language"]), []).append(program)
        if previous.get("loc") != count:
            changed.append(dict(key=row["key"],before=previous.get("loc"),after=count))
    for benchmark in result["benchmarks"]:
        present={g["language"] for g in benchmark["groups"]}
        for (key,language),rows in candidates.items():
            if key==benchmark["key"] and language not in present:
                label=next((l["label"] for l in result["languages"] if l["key"]==language),language)
                benchmark["groups"].append(dict(language=language,label=label,programs=[]))
                present.add(language)
        for group in benchmark["groups"]:
            rows = candidates.get((benchmark["key"],group["language"]))
            if rows is None:
                continue
            fast=min(rows,key=lambda p:(p["seconds"],p["loc"],p["key"]))
            small=min(rows,key=lambda p:(p["loc"],p["seconds"],p["key"]))
            group.update(fast=fast["key"],small=small["key"],
                programs=list(dict.fromkeys((fast["key"],small["key"]))))
            for program in (fast,small):
                result["programs"][program["key"]]=program
    for row in audit.get("current_source_reconciliation",[]):
        if row["key"] not in result["programs"]:continue
        program=result["programs"][row["key"]]
        count=sum(decisions[identity]["normalized_loc"] for identity in row["source_identities"])
        program.setdefault("producer_loc",row["producer_loc"])
        program["loc"]=count
        program["loc_metric"]="reviewed-canonical-source-units"
        program["loc_audit"]=COHORT
    def get_path(value,path):
        for part in path:value=value[part]
        return value
    def set_path(value,path,new):
        for part in path[:-1]:value=value[part]
        value[path[-1]]=new
    denominator_counts={row["sha256"]:sum(decisions[i]["normalized_loc"] for i in row["source_identities"])
        for row in audit.get("paired_control_source_reconciliation",[])}
    for comparison in audit.get("displayed_comparisons",{}).get("comparisons",[]):
        key=comparison["program_key"]
        if key not in result["programs"]:continue
        program=result["programs"][key]
        program["producer_mech_comparison"]=copy.deepcopy(program["mech_comparison"])
        for binding in comparison["bindings"]:
            denominator=denominator_counts[binding["denominator_source"]["sha256"]]
            set_path(result,binding["field_path"],denominator)
            if binding["ratio_path"]:set_path(result,binding["ratio_path"],program["loc"]/denominator)
        program["mech_comparison"]["loc_metric"]="reviewed-canonical-source-units"
        program["mech_comparison"]["loc_audit"]=COHORT
    for benchmark in result["benchmarks"]:
        # Mech modes and auxiliary paired numerators retain their admitted
        # measured pool, reselecting only from those existing program identities.
        for group in benchmark["groups"]:
            if (benchmark["key"],group["language"]) in candidates:continue
            rows=[result["programs"][key] for key in group["programs"]]
            timed=[p for p in rows if p.get("seconds") is not None]
            if timed:
                group["fast"]=min(timed,key=lambda p:(p["seconds"],p["loc"],p["key"]))["key"]
                group["small"]=min(timed,key=lambda p:(p["loc"],p["seconds"],p["key"]))["key"]
                group["programs"]=list(dict.fromkeys((group["fast"],group["small"])))
        retained=[result["programs"][key] for group in benchmark["groups"] for key in group["programs"]]
        benchmark["baseline_loc"]=min(p["loc"] for p in retained)
        benchmark["baseline_seconds"]=min(p["seconds"] for p in retained if p["seconds"] is not None)
        for program in retained:
            program["x"]=program["loc"]/benchmark["baseline_loc"]
            program["y"]=program["seconds"]/benchmark["baseline_seconds"] if program["seconds"] is not None else None
    retained={key for b in result["benchmarks"] for g in b["groups"] for key in g["programs"]}
    result["programs"]={key:p for key,p in result["programs"].items() if key in retained}
    retained_files={key for p in result["programs"].values() for key in
        [*p["files"],*p.get('fresh_paired_mech_control',{}).get('files',[])]}
    result["files"]={key:f for key,f in result["files"].items() if key in retained_files}
    result["reference_summary"]=[]
    for language in result["languages"]:
        inputs=[g for b in result["benchmarks"] for g in b["groups"]
            if g["language"]==language["key"] and g.get("fast") and result["programs"][g["small"]]["seconds"] is not None]
        if len(inputs)!=len(result["benchmarks"]):
            continue
        result["reference_summary"].append(dict(lang=language["key"],label=language["label"],
            **{kind:{axis:math.exp(sum(math.log(result["programs"][g[kind]][axis]) for g in inputs)/len(inputs))
                for axis in ("x","y")} for kind in ("fast","small")}))
    result.setdefault("provenance", {})[COHORT+"/reviewed-audit.json"]=sha(audit_path)
    result["legacy_loc_audit"]=dict(rule_version=audit["rule_version"],changed=changed,
        measured_candidate_count=len(audit["candidates"]),source_count=len(audit["sources"]),
        unchanged_timing_samples=True,original_reports_preserved=True)
    return result
