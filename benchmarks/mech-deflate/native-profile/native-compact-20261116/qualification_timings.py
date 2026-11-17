"""Replay paired benchmark measurements under the declared qualification policy."""

from collections import Counter, defaultdict
from itertools import permutations
from math import exp, log, sqrt
from random import Random
from statistics import fmean, median

from qualification_records import (
    ACTORS, CANDIDATE, CONTROL, MECH, PEERS, Findings, close, fixture_contract, is_count,
    is_digest, is_number, reference_sha, require,
)

DUPLICATE_ACTORS = ("control", "duplicate-control", "candidate")
CLEANUP = {
    CONTROL: "CLEANUP private_payload_bytes=0 live_owner_records=0",
    CANDIDATE: "CLEANUP private_payload_bytes=0 live_owner_records=0",
    "roc": "CLEANUP reference_input_and_output_roots_released=1",
    "libdeflate": "CLEANUP libdeflate_compressor_and_output_released=1",
}
COUNTERS = (
    "payload_allocation_requests", "requested_payload_bytes", "peak_private_payload_bytes",
    "zeroed_bytes", "initialized_bytes_copied",
)


def estimate(strata):
    return exp(sum(map(sum, strata)) / sum(map(len, strata))) - 1


def interval(strata, seed, policy):
    """Resample whole blocks within each run and actor-order stratum."""
    center = estimate(strata)
    low = min(map(min, strata))
    high = max(map(max, strata))
    if low == high:
        return {"estimate": center, "lower": center, "upper": center}
    rng = Random(seed)
    resamples = policy["bootstrap_resamples"]
    tail = (1 - policy["bootstrap_coverage"]) / 2
    total = sum(map(len, strata))
    draws = sorted(
        exp(sum(sum(rng.choices(part, k=len(part))) for part in strata) / total) - 1
        for _ in range(resamples)
    )
    return {"estimate": center, "lower": draws[round(tail * resamples)],
            "upper": draws[round((1 - tail) * resamples)]}


def paired(blocks, numerator, denominator):
    parts = defaultdict(list)
    for order, means in blocks:
        denominator_first = order.index(denominator) < order.index(numerator)
        parts[denominator_first].append(log(means[numerator] / means[denominator]))
    return [parts[True], parts[False]]


def verify_call(row, fixture, actor, *, timed):
    require(row["case"] == fixture["name"], "observation belongs to a different case")
    for field in ("input_bytes", "prefix_bytes"):
        require(row[field] == fixture[field], f"observed {field} differs from fixture")
    for field in ("independent_inflation", "input_alias_unchanged", "borrowed_output_alias_unchanged"):
        require(row[field] is True, f"{field} failed")
    if timed:
        require(row["warmup"] is False and row["profile"] is False, "warmup or profile call in timing blocks")
        require(is_number(row["seconds"]), "invalid duration")
    expected_digest = fixture.get("expected_raw_sha256")
    if actor == "libdeflate":
        require(row["level"] == 12 and row["version"] == "1.26", "unexpected libdeflate build or compression level")
        require(row["reference_capacity"] == fixture["capacity"]
                and is_count(row["capacity"], positive=True) and row["capacity"] >= fixture["capacity"]
                and is_count(row["raw_bytes"], positive=True), "invalid libdeflate output size or capacity")
        require(is_digest(row["output_sha256"]) and type(row["matches_reference_bytes"]) is bool,
                "invalid libdeflate output identity")
        if expected_digest is not None:
            require(row["matches_reference_bytes"] == (row["output_sha256"] == expected_digest),
                    "libdeflate exact-output claim disagrees with its digest")
    else:
        require(row["exact_reference"] is True and row["capacity"] == fixture["capacity"]
                and row["raw_bytes"] == fixture["raw_bytes"], "exact output or output capacity differs")
        if expected_digest is not None and "output_sha256" in row:
            require(row["output_sha256"] == expected_digest, "output digest differs from the exact reference")
    if actor in MECH:
        require(row["nonreturned_private_bytes_at_return"] == 0, "unreleased private allocation at return")
        for counter in COUNTERS:
            require(is_count(row[counter]), f"missing or invalid ownership counter {counter}")


def case_blocks(raw, fixture, expected_count):
    require(isinstance(raw, list) and len(raw) == expected_count, "block count differs from the complete receipt")
    counts = Counter()
    timings = {actor: [] for actor in ACTORS}
    peak = dict.fromkeys(MECH, 0)
    output = set()
    blocks = []
    batch = None
    for block in raw:
        order = tuple(block["order"])
        require(len(order) == len(ACTORS) and set(order) == set(ACTORS), "block lacks a permutation of all actors")
        counts[order] += 1
        require(set(block["observations"]) == set(ACTORS) == set(block["means"]), "block actor sets differ")
        means = {}
        for actor in ACTORS:
            rows = block["observations"][actor]
            require(isinstance(rows, list) and bool(rows), "empty actor batch")
            if batch is None:
                batch = len(rows)
            require(len(rows) == batch, "batch size changes within the case")
            samples = []
            for row in rows:
                verify_call(row, fixture, actor, timed=True)
                samples.append(row["seconds"])
                if actor in MECH:
                    peak[actor] = max(peak[actor], row["peak_private_payload_bytes"])
                if actor == "libdeflate":
                    output.add((row["output_sha256"], row["capacity"], row["raw_bytes"], row["matches_reference_bytes"]))
            timings[actor].extend(samples)
            means[actor] = fmean(samples)
            require(means[actor] > 0 and close(block["means"][actor], means[actor]),
                    "nonpositive block mean or mean disagrees with raw calls")
        blocks.append((order, means))
    require(len(counts) == len(set(permutations(ACTORS))) and len(set(counts.values())) == 1,
            "actor permutations are missing or unbalanced")
    require(len(output) == 1, "libdeflate output identity varies within the case")
    return blocks, timings, peak, output.pop()


def verify_summary(summary, fixture, timings, blocks, c_output):
    digest, capacity, raw_bytes, exact = c_output
    calculated = {
        "input_bytes": fixture["input_bytes"],
        "reference_raw_bytes": fixture["raw_bytes"],
        "reference_capacity": fixture["capacity"],
        "libdeflate_raw_bytes": raw_bytes,
        "libdeflate_capacity": capacity,
        "libdeflate_same_bytes": exact,
        "raw_stream_sha256": digest,
        "timed_calls": sum(map(len, timings.values())),
    }
    require(all(summary[key] == value for key, value in calculated.items()),
            "run summary disagrees with observed call counts, output, or fixture")
    require(set(summary["median_seconds"]) == set(ACTORS)
            and all(close(summary["median_seconds"][a], median(timings[a])) for a in ACTORS),
            "run medians disagree with raw calls")
    peer_ratios = {}
    for peer in PEERS:
        values = {
            actor: exp(fmean(log(means[actor] / means[peer]) for _, means in blocks))
            for actor in ACTORS
        }
        require(set(summary[f"paired_x_{peer}"]) == set(ACTORS)
                and all(close(summary[f"paired_x_{peer}"][a], values[a]) for a in ACTORS),
                f"paired {peer} summary disagrees with raw blocks")
        peer_ratios[peer] = values
    return peer_ratios


def check_run_pins(pins, run, fixtures, evidence, identities, policy):
    findings = evidence.findings
    require(set(pins["actors"]) == set(ACTORS), "pinned actor set differs")
    for actor, record in pins["actors"].items():
        for field, expected in (
            ("binary_sha256", identities[actor].get("worker")),
            ("metadata_sha256", identities[actor].get("metadata")),
        ):
            if expected is not None:
                require(record.get(field) == expected, f"{actor} {field} differs from the qualified artifact")
    for field in ("environment_sha256", "contract_sha256"):
        recorded = pins.get(field)
        if recorded is None or identities[field] is None:
            findings.unresolved.append(f"{run['id']}: missing run binding for {field}")
        else:
            require(recorded == identities[field], f"{field} differs from the qualified campaign")
    require(pins.get("level") == 12 and pins.get("fresh_libdeflate_compressor_inside_timing") is True,
            "runtime receipt changed the compression level or fresh-allocation timing boundary")
    frozen = policy["fixture_inventory_sha256"]
    if pins.get("input_inventory_sha256") != frozen:
        reference = run.get("fixture_inventory")
        inventory = evidence.json(reference, f"{run['id']} timed fixture inventory")
        if inventory is not None:
            require(reference_sha(reference) == pins.get("input_inventory_sha256"),
                    "timed fixture inventory hash disagrees with pins")
            require([row["name"] for row in inventory] == list(fixtures)
                    and all(fixture_contract(row) == fixture_contract(fixtures[row["name"]])
                            for row in inventory), "timed fixture contract differs from the frozen inventory")


def check_cleanup(run, evidence):
    for actor, footer in CLEANUP.items():
        path = evidence.path(run.get("cleanup", {}).get(actor), f"{run['id']} {actor} cleanup")
        if path is not None:
            require(path.read_text(encoding="utf-8").rstrip().endswith(footer), f"{actor} cleanup failed")


def runtime_run(run, fixtures, evidence, identities, policy, plan_entry=None):
    run_id = run["id"]
    summary = evidence.json(run.get("summary"), f"{run_id} summary")
    complete = evidence.json(run.get("complete"), f"{run_id} completion record")
    pins = evidence.json(run.get("pins"), f"{run_id} pins")
    if summary is None or complete is None or pins is None:
        return None
    require(run["order"] in ("forward", "reverse") and run["kind"] in ("full", "focus"),
            "unknown timing order or run kind")
    names = [row["case"] for row in summary]
    require(bool(names) and len(names) == len(set(names)), "missing or duplicate case in summary")
    require(set(names) <= fixtures.keys() and set(names) == run["observations"].keys(),
            "summary and raw observations contain unknown or mismatched cases")
    if run["kind"] == "full":
        require(set(names) == fixtures.keys(), "declared full timing order does not contain the exact frozen case suite")
    order = list(fixtures) if run["order"] == "forward" else list(reversed(fixtures))
    require(names == [name for name in order if name in run["observations"]], "case order does not match declared direction")
    expected_count = complete["balanced_blocks_per_case"]
    require(is_count(expected_count, positive=True), "invalid declared block count")
    if plan_entry is not None:
        require(plan_entry["order"] == run["order"] and plan_entry["kind"] == run["kind"]
                and plan_entry["blocks_per_case"] == expected_count, "timing run differs from the predeclared plan")
    check_run_pins(pins, run, fixtures, evidence, identities, policy)
    check_cleanup(run, evidence)
    ratios = {}
    paired_blocks = {}
    output_identities = {}
    peak = dict.fromkeys(MECH, 0)
    observed_counts = Counter()
    for row in summary:
        name = row["case"]
        raw = evidence.json(run["observations"][name], f"{run_id}/{name} observations")
        if raw is None:
            continue
        try:
            blocks, calls, case_peak, output = case_blocks(raw, fixtures[name], expected_count)
            ratios[name] = verify_summary(row, fixtures[name], calls, blocks, output)
        except (ValueError, KeyError, TypeError) as error:
            raise ValueError(f"{name}: {error}") from error
        paired_blocks[name] = blocks
        output_identities[name] = output
        observed_counts.update({
            "cases": 1,
            "timed_calls": sum(map(len, calls.values())),
            "reference_capacity_matches": int(output[1] == fixtures[name]["capacity"]),
            "libdeflate_exact_bytes_cases": int(output[3]),
        })
        for actor in MECH:
            peak[actor] = max(peak[actor], case_peak[actor])
    if len(paired_blocks) != len(names):
        return None
    require(all(complete[key] == value for key, value in observed_counts.items()),
            "completion counts disagree with observed calls")
    for field in ("fresh_compression_all_cases", "all_outputs_independently_inflated",
                  "all_input_and_output_aliases_preserved"):
        require(complete[field] is True, f"{field} failed")
    return {"id": run_id, "kind": run["kind"], "order": run["order"], "cases": paired_blocks,
            "ratios": ratios, "peak": peak, "peer_output": output_identities}


def report_run(run, policy, contracts):
    peer_results = {}
    for peer in PEERS:
        points = {actor: [] for actor in MECH}
        precision = {actor: [] for actor in MECH}
        for index, (name, ratios) in enumerate(run["ratios"].items()):
            for actor in MECH:
                if ratios[peer][actor] >= 1.0:
                    continue
                points[actor].append(name)
                parts = paired(run["cases"][name], actor, peer)
                largest = max(map(max, parts))
                if largest < 0 or interval(parts, index, policy)["upper"] < 0:
                    precision[actor].append(name)
        peer_results[peer] = {
            "point_estimate_wins": {actor: len(points[actor]) for actor in MECH},
            "precision_supported_wins": {actor: len(precision[actor]) for actor in MECH},
            "lost_cases": sorted(set(points[CONTROL]) - set(points[CANDIDATE])),
            "gained_cases": sorted(set(points[CANDIDATE]) - set(points[CONTROL])),
        }
        if run["kind"] == "full":
            peer_results[peer]["distinct_contract_point_estimate_wins"] = {
                actor: count_contract_wins(contracts, points[actor]) for actor in MECH
            }
            peer_results[peer]["distinct_contract_precision_supported_wins"] = {
                actor: count_contract_wins(contracts, precision[actor]) for actor in MECH
            }
    return {"id": run["id"], "kind": run["kind"], "order": run["order"],
            "case_count": len(run["cases"]), "peers": peer_results}


def count_contract_wins(contracts, cases):
    won = set(cases)
    return sum(group <= won for group in contracts.values())


def duplicate_study(entry, fixtures, evidence, identities, policy):
    study_id = entry["id"]
    study = evidence.json(entry.get("study"), f"{study_id} repeatability design")
    results = evidence.json(entry.get("results"), f"{study_id} repeatability summary")
    if study is None or results is None:
        return {}
    expected = {
        "control": identities[CONTROL].get("worker"),
        "duplicate-control": identities[CONTROL].get("worker"),
        "candidate": identities[CANDIDATE].get("worker"),
    }
    require(study["worker_sha256"] == expected, "duplicate-control worker identity differs")
    require(study["distinct_worker_files"] == 3, "duplicate-control study did not use distinct worker files")
    require(study["acceptance_resolution"] == policy["runtime_resolution"], "repeatability resolved at a different threshold")
    names = study["order"]
    require(len(names) == len(set(names)) and set(names) <= fixtures.keys()
            and set(names) == results.keys() == entry["observations"].keys(),
            "repeatability observations, cases, and reported results differ")
    all_orders = set(permutations(DUPLICATE_ACTORS))
    for field in ("environment_sha256", "contract_sha256"):
        if study.get(field) is not None:
            require(study[field] == identities[field], f"repeatability {field} differs")
        else:
            evidence.findings.unresolved.append(f"{study_id}: missing repeatability binding for {field}")
    report = {}
    case_indices = {name: index for index, name in enumerate(fixtures)}
    for name in names:
        raw = evidence.json(entry["observations"][name], f"{study_id}/{name} repeatability observations")
        if raw is None:
            continue
        batch = raw["batch"]
        require(raw["case"] == name and batch == results[name]["batch"]
                and is_count(batch, positive=True) and batch >= study["batch_floor"],
                f"{name}: mismatched repeatability case or batch")
        contrasts = {"aa": defaultdict(list), "ab": defaultdict(list)}
        for block in raw["blocks"]:
            order = tuple(block["order"])
            require(order in all_orders and set(block["seconds"]) == set(DUPLICATE_ACTORS)
                    and set(block["means"]) == set(DUPLICATE_ACTORS), f"{name}: invalid repeatability actor order")
            means = []
            for actor in DUPLICATE_ACTORS:
                samples = block["seconds"][actor]
                require(len(samples) == batch and all(is_number(v) for v in samples),
                        f"{name}: invalid repeatability sample batch")
                mean = fmean(samples)
                require(mean > 0 and close(block["means"][actor], mean),
                        f"{name}: nonpositive repeatability mean or mean disagrees with raw calls")
                means.append(mean)
            control, duplicate, candidate = means
            contrasts["aa"][order].append(log(duplicate / control))
            contrasts["ab"][order].append(log(candidate / sqrt(control * duplicate)))
        require(contrasts["aa"].keys() == all_orders
                and all(len(part) == study["repetitions_per_order"] for part in contrasts["aa"].values()),
                f"{name}: repeatability orders are incomplete or unbalanced")
        for contrast, parts in contrasts.items():
            actual = interval(list(parts.values()), study["seed"] + case_indices[name], policy)
            require(all(close(results[name][contrast][key], value) for key, value in actual.items()),
                    f"{name}: repeatability interval disagrees with raw blocks")
            report.setdefault(name, {})[contrast] = actual
    return report


def decide_runtime(runs, repeats, policy, fixtures, findings):
    resolution = policy["runtime_resolution"]
    complete = [run for run in runs if run["kind"] == "full"]
    contracts = defaultdict(set)
    for name, fixture in fixtures.items():
        contracts[fixture_contract(fixture)].add(name)
    reports = [report_run(run, policy, contracts) for run in runs]
    output_ids = {}
    focus_losses = Counter()
    for run, report in zip(runs, reports, strict=True):
        for name, identity in run["peer_output"].items():
            if output_ids.setdefault(name, identity) != identity:
                findings.rejected.append(f"{name}: libdeflate output identity differs across timing orders")
        for peer, scores in report["peers"].items():
            wins = scores["point_estimate_wins"]
            lost = scores["lost_cases"]
            if run["kind"] == "focus":
                focus_losses.update((peer, name) for name in lost)
                if lost:
                    findings.unresolved.append(f"{run['id']}: focused losses against {peer}: {', '.join(lost)}")
                continue
            if lost:
                findings.rejected.append(f"{run['id']}: lost previously won {peer} cases: {', '.join(lost)}")
            required = policy.get("required_peer_wins", {}).get(peer, ())
            missed = [name for name in required if run["ratios"][name][peer][CANDIDATE] >= 1.0]
            if missed:
                findings.rejected.append(f"{run['id']}: missed required {peer} wins: {', '.join(missed)}")
            floor = policy["peer_win_floors"][peer]
            if wins[CANDIDATE] < floor:
                reason = f"{run['id']}: {peer} floor {floor} missed (control {wins[CONTROL]}, candidate {wins[CANDIDATE]})"
                result = findings.rejected if wins[CONTROL] >= floor else findings.unresolved
                result.append(reason)
    for (peer, name), count in focus_losses.items():
        if count > 1:
            findings.rejected.append(f"{name}: repeated focused loss against {peer}")
    if {run["order"] for run in complete} != {"forward", "reverse"}:
        findings.unresolved.append("both complete timing orders are required")
    timing_findings = findings if policy.get("require_case_nonregression", True) else Findings()
    per_case = {}
    repeat_report = defaultdict(list)
    for study_id, records in repeats:
        for name, record in records.items():
            repeat_report[name].append({"id": study_id, **record})
    for index, name in enumerate(fixtures):
        intervals = {}
        all_parts = []
        for run_index, run in enumerate(runs):
            if name in run["cases"]:
                parts = paired(run["cases"][name], CANDIDATE, CONTROL)
                intervals[run["id"]] = interval(parts, index + 1000 * run_index, policy)
                if run["kind"] == "full" or not complete:
                    all_parts.extend(parts)
        if not all_parts:
            continue
        combined = interval(all_parts, index + 2000, policy)
        repeatability = repeat_report[name]
        sustained = sum(row["estimate"] > resolution for row in intervals.values()) >= 2
        adverse_runs = [run_id for run_id, row in intervals.items() if row["lower"] > resolution]
        noisy = any(r["aa"]["lower"] < -resolution or r["aa"]["upper"] > resolution for r in repeatability)
        failing_repeat = not noisy and any(r["ab"]["lower"] > resolution for r in repeatability)
        if combined["lower"] > resolution or failing_repeat:
            state = "regression"
            timing_findings.rejected.append(f"{name}: established slowdown above the 0.5% resolution")
        else:
            resolved = bool(repeatability) and all(r["ab"]["upper"] <= resolution for r in repeatability)
            if adverse_runs:
                state = "unresolved"
                timing_findings.unresolved.append(f"{name}: combined and individual run bounds conflict: {', '.join(adverse_runs)}")
            elif noisy or (combined["upper"] > resolution or sustained) and not resolved:
                state = "unresolved"
                timing_findings.unresolved.append(f"{name}: regression bound or duplicate-control repeatability unresolved")
            else:
                state = "bounded"
        per_case[name] = {"decision": state, "runs": intervals, "combined": combined, "adverse_runs": adverse_runs,
                          "duplicate_controls": repeatability}
    report = {
        "runs": reports, "cases": per_case,
        "scope": "90% per-case paired bootstrap, stratified by run and actor order",
        "equivalent_named_cases": [sorted(group) for group in contracts.values() if len(group) > 1],
        "peak_private_payload_bytes": (
            {actor: max(run["peak"][actor] for run in complete) for actor in MECH} if complete else {}
        ),
    }
    if timing_findings is not findings:
        report["candidate_control_timing_observations"] = {
            "slowdowns": timing_findings.rejected, "uncertain": timing_findings.unresolved,
        }
    return report
