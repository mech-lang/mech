#!/usr/bin/env python3
"""Produce a new review artifact under the shared timing mutex; never publish."""
import argparse
import fcntl
import hashlib
import json
import math
import os
import statistics
import sys
from contextlib import contextmanager
from datetime import datetime, timezone
from pathlib import Path

from normalizer import RULE_VERSION, normalize
from pools import collect_pool
from comparisons import collect_display_comparisons
from prior_array_loc import preserve as preserve_prior_array
from apl_closer_approved import apply_approved_closers
from foreign_dispatch import CURRENT_FAMILIES, PLANNED_FAMILIES, family_of, normalize_foreign
import pending_minima
import pending_minima_records

HERE = Path(__file__).resolve().parent
LOCK = Path("/tmp/mech-clbg-timing.lock")


@contextmanager
def timing_boundary():
    with LOCK.open("a+") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise RuntimeError("Timing is active; audit execution must wait for a common-mutex boundary.") from None
        yield lock


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def reconcile_completed_lua_studies(root,pool):
    """Independently validate every full profile, only in mutex-held execution."""
    if not pool.get("uncovered_reports"):
        return {}
    # A queue freezes these preexisting validator dependencies beside this
    # audit. Never resolve a live explorer helper during a frozen source pass.
    import isolated_study_data
    import mandelbrot_study_data
    used_dependencies={}
    for dependency in (isolated_study_data,mandelbrot_study_data):
        path=Path(dependency.__file__).resolve()
        if path.parent != HERE:
            raise ValueError('Completed-study validator was not frozen beside this audit: '+str(path))
        used_dependencies[str(path)]=digest(path)
    validate_full_records=isolated_study_data.validate_full_records
    resolved=[]
    for uncovered in pool["uncovered_reports"]:
        relative=uncovered["report"] if isinstance(uncovered,dict) else uncovered
        report=json.loads((root/relative).read_text())
        cohort=Path(relative).parents[1]
        manifest_name=str(cohort/"full-study/manifest.json")
        manifest=json.loads((root/manifest_name).read_text())
        reference_path=root/"pidigits/inputs/output-full.txt"
        reference=dict(bytes=reference_path.stat().st_size,sha256=digest(reference_path))
        for path,expected in manifest["pins"].items():
            if digest(path)!=expected:
                raise ValueError("Completed Lua study frozen pin changed: "+path)
        records,discarded=validate_full_records(manifest,report,digest(root/manifest_name),
            benchmark="pidigits",size=10000,
            locs={row["id"]:row["loc"] for row in manifest["profiles"]},
            workers=1,reference=reference,environment_retries=True)
        for row in records.values():
            if row["language"] not in {"lua","luajit"}:
                continue
            candidate=dict(row,original_loc=row["loc"],original_physical_loc=row.get("original_physical_loc"),
                samples=row["samples_seconds"],seconds=row["median_seconds"],
                report=relative,source_report=manifest_name,
                source_files=[dict(path=row["source"],sha256=row["source_sha256"])],
                source_identities=[],already_audited=True,authoritative_loc=row["loc"],
                aliases=[],current_keys=[],
                qualification=dict(admitted=True,report_completed=True,full_workload=True,
                    sample_count=3,declared_sample_count=3,sample_count_matches=True,
                    full_output_validated=True,validation_basis="independent complete durable full-record reconciliation",discarded_environment_records=len(discarded)),
                runtime_policy="LuaJIT" if row["language"]=="luajit" else "Lua interpreted",
                execution_mode="jit" if row["language"]=="luajit" else "interpreted")
            pool["candidates"]=[p for p in pool["candidates"] if p["key"]!=candidate["key"]]
            pool["candidates"].append(candidate)
            pool["pending_candidates"]=[p for p in pool["pending_candidates"] if p["key"]!=candidate["key"]]
        for name in (manifest_name,relative):
            if name not in pool["report_paths"]:pool["report_paths"].append(name)
        resolved.append(dict(report=relative,status="all-complete-profiles-independently-reconciled",profiles=len(records),discarded_environment_records=len(discarded)))
    pool["reconciliation"]=[p for p in pool["reconciliation"] if p.get("status")!="uncovered-complete-report"]+resolved
    pool["uncovered_reports"]=[]
    pool["coverage_complete"]=True
    pool["candidates"].sort(key=lambda p:p["key"])
    return used_dependencies


def produce(root, current_data_path):
    import pygments
    normalization_environment = dict(python_executable=sys.executable,
        python_version=sys.version, pygments_version=pygments.__version__,
        pygments_module=str(Path(pygments.__file__).resolve()),
        pygments_module_sha256=digest(pygments.__file__))
    current = json.loads(current_data_path.read_text())
    pool = collect_pool(root, current)
    study_tool_pins=reconcile_completed_lua_studies(root,pool)
    for dependency in (pending_minima,pending_minima_records):
        if Path(dependency.__file__).resolve().parent != HERE:
            raise ValueError('Pending-minimum adapter was not frozen beside this audit.')
    pending_minima.reconcile(root,pool,digest=digest,
        validate_study=pending_minima_records.validate_study)
    display=collect_display_comparisons(root,current)
    sources = {}
    prior_provenance_pins = {}
    approval_relative = 'loc-normalization/20261012/apl-closer-approval-job07.json'
    approval_path = root / approval_relative
    frozen_approval = HERE / 'apl-closer-approval-job07.json'
    # Frozen jobs copy the additive approval beside their parser snapshot.
    # Original producer/source/manifest pins are still reconciled below first.
    if not approval_path.is_file() or not frozen_approval.is_file():
        raise ValueError('Exact six-source additive APL approval receipt is missing.')
    approval_pin = digest(frozen_approval)
    if digest(approval_path) != approval_pin:
        raise ValueError('Additive APL approval differs from its frozen parser-job snapshot.')
    approval = json.loads(frozen_approval.read_text())
    prior_provenance_pins[approval_relative] = approval_pin
    policy_relative = 'loc-normalization/20261012/policy-scope-correction-20261012.json'
    policy_path = root / policy_relative
    frozen_policy = HERE / 'policy-scope-correction-20261012.json'
    if not policy_path.is_file() or not frozen_policy.is_file() or digest(policy_path) != digest(frozen_policy):
        raise ValueError('Additive canonical scope policy differs from its frozen parser-job snapshot.')
    prior_provenance_pins[policy_relative] = digest(frozen_policy)
    initialization_relative='loc-normalization/20261012/policy-explicit-initialization-20261012.json'
    initialization_path=root/initialization_relative
    frozen_initialization=HERE/'policy-explicit-initialization-20261012.json'
    if not initialization_path.is_file() or not frozen_initialization.is_file() or digest(initialization_path)!=digest(frozen_initialization):
        raise ValueError('Additive explicit-initialization policy differs from its frozen parser-job snapshot.')
    prior_provenance_pins[initialization_relative]=digest(frozen_initialization)
    diagnostics = [dict(kind="complete-study-pool-not-yet-reconciled",report=report)
        for report in pool.get("uncovered_reports",[])]

    def count(candidate):
        counts = []
        identities = []
        issues = []
        for ref in candidate["source_files"]:
            path = Path(ref["path"])
            if not path.is_file():
                issues.append(dict(kind="missing-exact-source", path=str(path)))
                continue
            raw=path.read_bytes()
            actual = hashlib.sha256(raw).hexdigest()
            if not ref.get("sha256") or actual != ref["sha256"]:
                issues.append(dict(kind="source-identity-mismatch", path=str(path),
                    expected=ref.get("sha256"), actual=actual))
                continue
            language = {"luajit":"lua","mech-jit":"mech","mech-resident":"mech"}.get(candidate["language"],candidate["language"])
            identity = actual + ":" + language
            if identity not in sources:
                text = raw.decode("utf-8")
                metric = None
                current_program = candidate.get('current_program')
                if current_program is not None and language in {'apl', 'uiua'}:
                    producer_path = root / current_program['source_report']
                    producer_pin = digest(producer_path)
                    metric = preserve_prior_array(text, language, actual, current_program, root)
                    if digest(producer_path) != producer_pin:
                        raise ValueError('Prior array producer changed during source reconciliation.')
                    if metric is not None:
                        provenance = metric['provenance']
                        if producer_pin != provenance['expected_producer_sha256']:
                            raise ValueError('Prior array producer differs from its preexisting archived pin.')
                        provenance['producer_report_sha256'] = producer_pin
                        for pinned_path, expected_pin in provenance['provenance_pins'].items():
                            if digest(root / pinned_path) != expected_pin:
                                raise ValueError('Prior array provenance differs from its frozen pin: '+pinned_path)
                            prior_provenance_pins[pinned_path] = expected_pin
                        approved = apply_approved_closers(metric, approval, approval_path, approval_pin)
                        if approved is not None:
                            metric = approved
                if metric is None:
                    metric = (normalize_foreign(text, language) if family_of(language) in CURRENT_FAMILIES | PLANNED_FAMILIES
                              else normalize(text, language))
                sources[identity] = dict(metric, sha256=actual, language=language, paths=[str(path)])
            elif str(path) not in sources[identity]["paths"]:
                sources[identity]["paths"].append(str(path))
            counts.append(sources[identity])
            identities.append(identity)
            issues.extend(dict(issue, source_sha256=actual) for issue in sources[identity]["diagnostics"])
        if not candidate["source_files"]:
            issues.append(dict(kind="missing-source-metadata"))
        if len(counts) != len(candidate["source_files"]):
            return dict(candidate, source_identities=identities, physical_loc=None,
                normalized_loc=None, review_status="needs-review", diagnostics=issues)
        physical = sum(row["physical_loc"] for row in counts)
        computed = (sum(row["normalized_loc"] for row in counts)
                    if all(row['normalized_loc'] is not None for row in counts) else None)
        normalized = computed
        if candidate.get("already_audited"):
            if candidate["authoritative_loc"] != computed:
                issues.append(dict(kind="previous-normalization-policy-change-review",
                    producer_normalized_loc=candidate["authoritative_loc"], canonical_parser_loc=computed))
        return dict(candidate, source_identities=identities, physical_loc=physical,
            candidate_parser_loc=computed, normalized_loc=normalized,
            original_to_normalized_delta=normalized-candidate["original_loc"] if normalized is not None and candidate.get("original_loc") is not None else None,
            review_status="needs-review" if issues else "candidate-parser-pass",
            diagnostics=issues)

    candidates = [count(candidate) for candidate in pool["candidates"]]
    pending = [count(candidate) for candidate in pool["pending_candidates"] if candidate.get("source_files")]
    current_sources=[]
    target_current={"mech","mech-jit","mech-resident"}
    paired_keys={row["program_key"] for row in display["comparisons"]}
    for key,row in current["programs"].items():
        if row["language"] not in target_current and key not in paired_keys:
            continue
        refs=[]
        for identity in row["files"]:
            file=current["files"][identity]
            if file.get("role")=="runtime-support":continue
            path=Path(file["path"])
            refs.append(dict(path=str(path if path.is_absolute() else root/path),sha256=file.get("sha256",identity)))
        existing=next((p for p in candidates if p["key"]==key),None)
        if existing:
            current_sources.append(dict(key=key,language=row["language"],source_identities=existing["source_identities"],producer_loc=row["loc"],canonical_parser_loc=existing["normalized_loc"],diagnostics=existing["diagnostics"]))
        else:
            counted=count(dict(key=key,language=row["language"],source_files=refs,original_loc=row["loc"],
                               current_program=dict(row,key=key)))
            current_sources.append(dict(key=key,language=row["language"],source_identities=counted["source_identities"],producer_loc=row["loc"],canonical_parser_loc=counted["normalized_loc"],diagnostics=counted["diagnostics"]))
    denominator_sources=[]
    for ref in display["source_files"]:
        counted=count(dict(key="paired-control:"+ref["sha256"],language="mech",source_files=[ref],original_loc=None))
        denominator_sources.append(dict(ref,source_identities=counted["source_identities"],canonical_parser_loc=counted["normalized_loc"],diagnostics=counted["diagnostics"]))
    fresh_control_sources=[]
    for row in candidates:
        control=row.get('fresh_paired_mech_control')
        if control is None:continue
        ref=dict(path=control['source'],sha256=control['source_sha256'])
        counted=count(dict(key='fresh-paired-control:'+row['key'],language='mech',
            source_files=[ref],original_loc=None))
        row['fresh_paired_mech_control_source_identities']=counted['source_identities']
        fresh_control_sources.append(dict(ref,candidate_key=row['key'],language='mech',
            source_identities=counted['source_identities'],canonical_parser_loc=counted['normalized_loc'],
            producer_loc=control['loc'],diagnostics=counted['diagnostics'],
            report=row['report'],samples=control['samples_seconds']))
    for row in current_sources:
        diagnostics.extend(dict(issue,key=row["key"],scope="current-displayed-source") for issue in row["diagnostics"])
    for row in denominator_sources:
        diagnostics.extend(dict(issue,source_sha256=row["sha256"],scope="historical-paired-control") for issue in row["diagnostics"])
    for row in fresh_control_sources:
        diagnostics.extend(dict(issue,key=row['candidate_key'],source_sha256=row['sha256'],
            scope='fresh-paired-control') for issue in row['diagnostics'])
    diagnostics.extend(dict(issue,kind="displayed-comparison-source-unresolved") for issue in display["unresolved"])
    groups = {}
    for row in candidates:
        groups.setdefault((row["benchmark"], row["language"]), []).append(row)
        diagnostics.extend(dict(issue, key=row["key"]) for issue in row["diagnostics"])
        if row["seconds"] != statistics.median(row["samples"]):
            diagnostics.append(dict(kind="median-reconciliation-needed", key=row["key"]))
    proposals = []
    pending_needed = []
    for (benchmark, language), rows in sorted(groups.items()):
        if any(row["normalized_loc"] is None for row in rows):
            proposals.append(dict(benchmark=benchmark, language=language,
                status="missing-source-count", candidate_keys=[row["key"] for row in rows]))
            continue
        fast = min(rows, key=lambda row:(row["seconds"], row["normalized_loc"], row["key"]))
        small = min(rows, key=lambda row:(row["normalized_loc"], row["seconds"], row["key"]))
        old_group = next((group for b in current["benchmarks"] if b["key"] == benchmark
            for group in b["groups"] if group["language"] == language), {})
        for row in pending:
            if (row["benchmark"], row["language"]) != (benchmark, language):
                continue
            if row.get("pending_classification") in {"unsupported", "work-excluded", "historically-excluded"}:
                continue
            uncertain_possible=row["review_status"]!="candidate-parser-pass"
            if (row["normalized_loc"] is not None and row["normalized_loc"] <= small["normalized_loc"]) or uncertain_possible:
                pending_needed.append(dict(key=row["key"], benchmark=benchmark, language=language,
                    normalized_loc=row["normalized_loc"], measured_smallest_key=small["key"],
                    measured_smallest_loc=small["normalized_loc"], report=row["report"],
                    status="parser-review-before-qualification-and-measurement" if uncertain_possible else "qualification-and-three-full-samples-required", reason=row["reason"],
                    parser_review_status=row["review_status"]))
        proposals.append(dict(benchmark=benchmark, language=language,
            status="needs-review" if any(row["review_status"] != "candidate-parser-pass" for row in rows) else "candidate-parser-pass",
            candidates=len(rows), candidate_keys=[row["key"] for row in rows],
            fastest_key=fast["key"], fastest_loc=fast["normalized_loc"], fastest_seconds=fast["seconds"],
            smallest_key=small["key"], smallest_loc=small["normalized_loc"], smallest_seconds=small["seconds"],
            previous_fastest_key=old_group.get("fast"), previous_smallest_key=old_group.get("small"),
            endpoint_changed=(fast["key"] != old_group.get("fast") or small["key"] != old_group.get("small"))))
    # Existing explicit current normalized rows are retained, never expanded twice.
    report_paths=list(dict.fromkeys([*pool["report_paths"],*display["report_paths"],*prior_provenance_pins]))
    report_pins = {relative:digest(root/relative) for relative in report_paths}
    for path, expected_pin in prior_provenance_pins.items():
        if report_pins.get(path) != expected_pin:
            raise ValueError('Prior array producer/manifest changed after source reconciliation: '+path)
    for comparison in display["comparisons"]:
        for binding in comparison["bindings"]:
            evidence=binding["denominator_source"].get("identity_evidence",{})
            snapshot=evidence.get("control_snapshot")
            expected=evidence.get("expected_snapshot_sha256")
            if snapshot and expected and report_pins.get(snapshot)!=expected:
                raise ValueError("Paired control snapshot differs from producer's frozen digest: "+snapshot)
    return dict(schema_version=1, status="candidate-audit-awaiting-review", rule_version=RULE_VERSION,
        created_at=datetime.now(timezone.utc).isoformat(), root=str(root),
        normalization_environment=normalization_environment,
        additive_policy_receipts={approval_relative:approval_pin,
                                  policy_relative:prior_provenance_pins[policy_relative],
                                  initialization_relative:prior_provenance_pins[initialization_relative]},
        execution=dict(common_timing_mutex=str(LOCK), exclusive_mutex_held=True,
            timing_processes_started=0, benchmark_source_bytes_modified=False),
        current_data=dict(path=str(current_data_path), sha256=digest(current_data_path)),
        current_legacy_projection={key:{field:row.get(field) for field in ("language","benchmark","loc","seconds","samples","files")}
            for key,row in current["programs"].items() if row["language"] in {"c","cpp","rust","go","java","chapel","julia","lua","luajit","python","ruby","perl","php","javascript","mech","mech-jit","mech-resident"} or key in paired_keys},
        report_pins=report_pins,
        tool_pins={str(path):digest(path) for path in [HERE/name for name in (
            'audit.py','normalizer.py','lexical.py','canonical.py','pools.py','comparisons.py',
            'prior_array_loc.py','apl_closer_approved.py','value_forms.py','foreign_dispatch.py',
            'c_type_environment.py',
            'compiler_attributes.py',
            'c_named_types.py',
            'pending_minima.py','pending_minima_records.py',
            'record_fields.py',
            'foreign_core.py','foreign_braced.py','zig_odin.py','foreign_keyword.py',
            'foreign_layout.py','foreign_smalltalk.py')]} | study_tool_pins,
        sources=sources, candidates=candidates, pending_candidates=pending,
        proposals=proposals, pending_needed_measurement=pending_needed,
        diagnostics=diagnostics, reconciliation=pool["reconciliation"],
        coverage_complete=pool.get("coverage_complete",True),
        current_source_reconciliation=current_sources,paired_control_source_reconciliation=denominator_sources,
        fresh_control_source_reconciliation=fresh_control_sources,
        displayed_comparisons=display,
        scope="All saved measured candidates in the reconstructed legacy pools, before endpoint selection. Unmeasured listed/prepared candidates remain unscored.",
        admission="No chart admission from this candidate artifact. Source-specific parser ambiguities and existing-audit reconciliations require review; prospective smaller unmeasured candidates require qualification and three full samples.")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=HERE.parents[1])
    parser.add_argument("--current-data", type=Path)
    parser.add_argument("--output", type=Path, default=HERE/"candidate-audit.json")
    args = parser.parse_args()
    root = args.root.resolve()
    current_data = (args.current_data or root/"overview/explorer/dist/data.json").resolve()
    if args.output.exists():
        raise SystemExit("Existing audit artifacts are immutable; choose a fresh --output.")
    with timing_boundary():
        result = produce(root, current_data)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False)+"\n")
        print(json.dumps(dict(output=str(args.output), status=result["status"],
            measured_candidates=len(result["candidates"]), source_identities=len(result["sources"]),
            diagnostics=len(result["diagnostics"]), pending_needed_measurement=len(result["pending_needed_measurement"]))))


if __name__ == "__main__":
    main()
