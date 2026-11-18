"""Validate provenance, proof observations, and separately scoped metrics."""

from collections import Counter
from math import log
from statistics import median
import re
import struct

from qualification_records import (
    ACTORS, CANDIDATE, CONTROL, MECH, close, is_count, is_number,
    read_json, reference_sha, require,
)
from qualification_timings import interval, verify_call

MECH_ARTIFACTS = ("source", "compiler", "bytecode", "object", "metadata", "runtime", "adapter", "worker", "options")


def artifacts_for(actor, record, evidence, identities, paths):
    fields = MECH_ARTIFACTS if actor in MECH else ("worker", "metadata") if actor == "roc" else ("worker",)
    for field in dict.fromkeys((*fields, *sorted(record.keys() & {"object", "build_executable"}))):
        ref = record.get(field)
        identities[actor][field] = reference_sha(ref)
        paths[actor][field] = evidence.path(ref, f"{actor} {field}")
    if actor not in MECH:
        return
    revision = record.get("source_commit")
    if revision is None:
        evidence.findings.unresolved.append(f"{actor}: compiler source revision missing")
    else:
        require(isinstance(revision, str) and re.fullmatch(r"[0-9a-f]{40}", revision),
                f"{actor}: source revision must be a full Git object ID")
    receipt = evidence.json(record.get("build_receipt"), f"{actor} compiler and link provenance")
    if receipt is not None:
        require(receipt["source_commit"] == revision and receipt["artifact_sha256"] == identities[actor],
                f"{actor}: build or link receipt disagrees with artifact identities")
        for command in ("frontend", "emit", "link"):
            require(isinstance(receipt["commands"][command], list) and bool(receipt["commands"][command]),
                    f"{actor}: missing {command} invocation")


def check_correctness(check, fixtures, evidence, identities):
    receipt = evidence.json(check.get("receipt"), "linked correctness receipt")
    calls = evidence.json(check.get("calls"), "linked correctness calls")
    if receipt is None or calls is None:
        return None
    require(receipt["verified"] is True, "linked correctness check failed")
    require(receipt["worker_sha256"] == identities[CANDIDATE].get("worker")
            and receipt["metadata_sha256"] == identities[CANDIDATE].get("metadata"),
            "linked correctness receipt belongs to a different worker or metadata")
    require(receipt["complete_smokes"] == len(calls), "linked correctness sample count differs")
    samples = Counter()
    exact_digests = True
    for row in calls:
        name = row["case"]
        require(name in fixtures, "unknown correctness case")
        verify_call(row, fixtures[name], CANDIDATE, timed=False)
        exact_digests &= "output_sha256" in row and "expected_raw_sha256" in fixtures[name]
        samples[name] += 1
    require(samples.keys() == fixtures.keys() and all(count >= 2 for count in samples.values()),
            "linked correctness report must test every frozen case twice")
    if not exact_digests:
        evidence.findings.unresolved.append("linked correctness calls lack exact raw output SHA-256 evidence")
    return {"cases": len(samples), "calls": len(calls), "exact_output_digests_verified": exact_digests}


def checked_subject(row, actor, evidence, identities, label):
    if row is None:
        evidence.findings.unresolved.append(f"{label}: {actor} proof subject is unbound")
        return
    for field in ("object", "metadata"):
        require(row[f"{field}_sha256"] == identities[actor].get(field),
                f"{label}: {actor} proved a different {field}")
    evidence.path(row.get("probe_worker"), f"{label} {actor} probe worker")


def check_failures(ref, paths, evidence, identities, policy):
    checked = evidence.json(ref, "checked-error observations")
    if checked is None:
        return None
    require(checked["cases"].keys() == policy["checked_failures"].keys(), "checked failure cases differ from the frozen set")
    subjects = checked.get("subjects", {})
    origins = {}
    for actor in MECH:
        checked_subject(subjects.get(actor), actor, evidence, identities, "checked errors")
        if paths[actor].get("metadata") is not None:
            origins[actor] = read_json(paths[actor]["metadata"])["origins"]
    for name, expected in policy["checked_failures"].items():
        record = checked["cases"][name]
        observed = record["per_actor_outcomes"]
        claimed_origins = record["matched_source_origins"]
        require(len(observed) == len(claimed_origins) == len(record["raw"]) == 2, f"{name}: missing paired failure observation")
        for actor, outcome, claimed, raw in zip(MECH, observed, claimed_origins, record["raw"], strict=True):
            require(outcome["case"] == name and outcome["status"] == expected["status"],
                    f"{name}: incorrect {actor} failure status or case")
            require(is_count(outcome["origin"]), f"{name}: invalid origin index")
            if actor in origins:
                metadata_origin = origins[actor][outcome["origin"]]
                require(metadata_origin == claimed, f"{name}: {actor} origin disagrees with emitted metadata")
            elif outcome["origin"] != expected["origin"]:
                evidence.findings.unresolved.append(f"{name}: changed {actor} origin cannot be verified without metadata")
            require((claimed["node"], claimed["local"]) == (expected["node"], expected["local"]),
                    f"{name}: incorrect {actor} semantic failure origin")
            require(bool(raw) and all(is_number(row["seconds"])
                                      and {key: value for key, value in row.items() if key != "seconds"} == outcome
                                      for row in raw), f"{name}: failure summary disagrees with raw calls")
        require(record["outcome"] == observed[0], f"{name}: control failure summary disagrees")
        if CONTROL in origins and CANDIDATE in origins and origins[CONTROL] == origins[CANDIDATE]:
            require(observed[0]["origin"] == observed[1]["origin"], f"{name}: failure origin changed on identical origin maps")
    return {"cases": len(checked["cases"])}


def check_stack(probes, fixtures, evidence, identities, plan):
    peaks = {}
    for role, actor in (("control", CONTROL), ("candidate", CANDIDATE)):
        label = f"{actor} full stack"
        record = evidence.json(probes.get(role), label)
        if record is None:
            continue
        subjects = record.get("subject")
        checked_subject(subjects, actor, evidence, identities, label)
        if "cases" in record:
            cases = record["cases"]
        else:
            require(len(record) == 1, f"{label}: ambiguous legacy probe")
            cases = next(iter(record.values()))
        require(cases.keys() == fixtures.keys(), f"{label}: probe does not cover every frozen case")
        samples = []
        for name, values in cases.items():
            require(set(values) == {"0", "1"} and all(is_count(n, positive=True) for n in values.values()),
                    f"{label}: invalid probe samples for {name}")
            samples.extend(values.values())
        if samples:
            peaks[actor] = max(samples)
    ceiling = plan.get("max_peak_stack_bytes")
    if ceiling is not None:
        require(is_count(ceiling, positive=True), "invalid predeclared stack limit")
        if CANDIDATE in peaks:
            require(peaks[CANDIDATE] <= ceiling, "measured peak stack exceeds the predeclared limit")
    return peaks


def macho_sizes(path):
    """Read the Apple ARM64 Mach-O text section and complete file size."""
    data = path.read_bytes()
    require(len(data) >= 32, "truncated Mach-O header")
    magic, cpu, _, _, command_count, command_bytes, _, _ = struct.unpack_from("<8I", data)
    require((magic, cpu) == (0xFEEDFACF, 0x0100000C), "expected Apple ARM64 Mach-O")
    offset = 32
    end = offset + command_bytes
    require(end <= len(data), "Mach-O load commands extend beyond the file")
    text = None
    for _ in range(command_count):
        require(offset + 8 <= end, "truncated Mach-O load command")
        kind, size = struct.unpack_from("<2I", data, offset)
        require(8 <= size <= end - offset, "invalid Mach-O load command size")
        if kind == 0x19:
            require(size >= 72, "truncated ARM64 segment")
            count = struct.unpack_from("<I", data, offset + 64)[0]
            require(72 + 80 * count <= size, "truncated Mach-O section list")
            for index in range(count):
                section = offset + 72 + index * 80
                if data[section:section + 32] == b"__text".ljust(16, b"\0") + b"__TEXT".ljust(16, b"\0"):
                    require(text is None, "duplicate Mach-O text sections")
                    text, file_offset = struct.unpack_from("<QI", data, section + 40)
                    require(text > 0 and file_offset + text <= len(data), "invalid Mach-O text range")
        offset += size
    require(offset == end and text is not None, "Mach-O text section missing")
    return text, len(data)


def collect_sizes(paths):
    sizes = {}
    for actor, artifacts in paths.items():
        row = {}
        if artifacts.get("object") is not None:
            row["encoder_text_bytes"], row["encoder_object_bytes"] = macho_sizes(artifacts["object"])
        linked = artifacts.get("build_executable") or artifacts.get("worker")
        if linked is not None:
            row["linked_text_bytes"], row["executable_file_bytes"] = macho_sizes(linked)
        if artifacts.get("metadata") is not None and actor in MECH:
            row["root_frame_bytes"] = read_json(artifacts["metadata"])["stack_bytes"]
        sizes[actor] = row
    return sizes


def match_trial(trial, order, identities):
    require(len(order) >= 2 and len(order) == len(set(order)) and set(MECH) <= set(order) <= set(ACTORS),
            "invalid measured build order")
    require(set(trial["seconds"]) == set(order) == set(trial["peak_rss_bytes"]) == set(trial["outputs"]),
            "build observations differ from the predeclared actor order")
    for actor in order:
        require(is_number(trial["seconds"][actor], positive=True)
                and is_count(trial["peak_rss_bytes"][actor], positive=True), "invalid build time or compiler peak RSS")
        worker = identities[actor].get("build_executable") or identities[actor].get("worker")
        obj = identities[actor].get("object")
        require(worker is not None and obj is not None
                and trial["outputs"][actor] == {"object_sha256": obj, "executable_sha256": worker},
                "complete build produced a different object or executable")


def check_builds(ref, evidence, identities, plan, policy):
    campaign = evidence.json(ref, "complete source-through-link build trials")
    if campaign is None:
        return {}
    require(campaign["scope"] == "source-through-final-link" and campaign["prebuilt_compilers_and_adapters"] is True,
            "build timing boundary differs from the source-through-link contract")
    require(campaign["environment_sha256"] == identities["environment_sha256"],
            "build timing environment differs from the campaign")
    trials = campaign["timed_trials"]
    orders = [trial["order"] for trial in trials]
    require(bool(orders), "no timed complete builds")
    frozen = plan.get("build_orders")
    if frozen is None:
        evidence.findings.unresolved.append("build order was not predeclared")
    else:
        require(orders == frozen, "complete build orders differ from the predeclared plan")
    for trial, order in zip(trials, orders, strict=True):
        match_trial(trial, order, identities)
    counts = Counter(tuple(order) for order in orders)
    require(len(counts) >= 2 and len(set(counts.values())) == 1
            and len({frozenset(order) for order in orders}) == 1, "unbalanced matched build orders")
    for actor in orders[0]:
        positions = Counter(order.index(actor) for order in orders)
        require(len(positions) == len(orders[0]) and len(set(positions.values())) == 1,
                f"{actor} does not occupy every build position equally")
    pair_order = build_strata(orders, trials)
    require(all(pair_order), "both Mech actor orders are required in the matched build comparison")
    return {
        "source_to_link_seconds": {actor: median(row["seconds"][actor] for row in trials) for actor in orders[0]},
        "compiler_peak_rss_bytes": {actor: max(row["peak_rss_bytes"][actor] for row in trials) for actor in orders[0]},
        "paired_change": interval(pair_order, 0, policy),
    }


def build_strata(orders, trials):
    parts = [[], []]
    for order, trial in zip(orders, trials, strict=True):
        seconds = trial["seconds"]
        before_first = order.index(CONTROL) < order.index(CANDIDATE)
        parts[int(before_first)].append(log(seconds[CANDIDATE] / seconds[CONTROL]))
    return parts
