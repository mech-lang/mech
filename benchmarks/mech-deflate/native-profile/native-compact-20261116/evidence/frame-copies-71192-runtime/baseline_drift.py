"""Compare the identical 71,768-byte worker across the two full campaigns."""

from statistics import median

from qualification_records import ACTORS, CANDIDATE, CONTROL, MECH, PEERS, Evidence, Findings, digest, read_json, require
from qualification_timings import case_blocks, interval, paired, verify_summary


def binding_digests(plan, actor):
    return {file: ref["sha256"] for file, ref in plan["bindings"][actor].items()}


def point_wins(rows, actor, peer):
    return {name for name, row in rows.items() if row[f"paired_x_{peer}"][actor] < 1.0}


def precision_exceptions(run, policy, peer, actor, result):
    if result["point_estimate_wins"][actor] == result["precision_supported_wins"][actor]:
        return []
    unsupported = []
    for index, (name, ratios) in enumerate(run["ratios"].items()):
        if ratios[peer][actor] < 1.0:
            parts = paired(run["cases"][name], actor, peer)
            if max(map(max, parts)) >= 0 and interval(parts, index, policy)["upper"] >= 0:
                unsupported.append(name)
    require(len(unsupported) == result["point_estimate_wins"][actor] - result["precision_supported_wins"][actor],
            "precision exception calculation differs from v1")
    return unsupported


def per_actor_durations(blocks, summary):
    quantile_index = round(0.9 * (len(blocks) - 1))
    reports = {}
    for actor in summary["median_seconds"]:
        times = sorted(means[actor] for _, means in blocks)
        reports[actor] = {
            "median_call_us": summary["median_seconds"][actor] * 1e6,
            "median_block_mean_us": median(times) * 1e6,
            "p90_block_mean_us": times[quantile_index] * 1e6,
            "maximum_block_mean_us": times[-1] * 1e6,
        }
    return reports


def timing_shift(old_blocks, old_row, run, row):
    blocks = run["cases"][row["case"]]
    old = per_actor_durations(old_blocks, old_row)
    new = per_actor_durations(blocks, row)
    previous_actor = {CONTROL: CANDIDATE, CANDIDATE: CANDIDATE, "roc": "roc", "libdeflate": "libdeflate"}
    threshold_seconds = {actor: 2 * old[previous_actor[actor]]["median_block_mean_us"] / 1e6 for actor in ACTORS}
    high_by_block = [sum(means[actor] > threshold_seconds[actor] for actor in ACTORS) for _, means in blocks]
    return {
        "order_position": list(run["cases"]).index(row["case"]) + 1,
        "all_blocks_per_campaign_and_order": len(blocks),
        "old_71768_and_peers": {actor: old[actor] for actor in (CANDIDATE, *PEERS)},
        "new_control_candidate_and_peers": new,
        "new_blocks_gt_2x_same_actor_historical_median": {
            actor: sum(means[actor] > threshold_seconds[actor] for _, means in blocks) for actor in ACTORS
        },
        "new_blocks_with_two_or_more_actors_gt_2x": sum(count >= 2 for count in high_by_block),
        "new_blocks_with_all_actors_gt_2x": high_by_block.count(len(ACTORS)),
    }


def compare_campaigns(archive, packet, plan, current, runs, fixtures, policy):
    previous = packet / "evidence" / "bound-71768-runtime"
    old_plan = read_json(previous / "plan.json")
    old_replay = read_json(previous / "full-replay.json")
    old_replay_sha256 = digest(previous / "full-replay.json")
    previous_followup = read_json(previous / "followup-plan.json")
    require(digest(previous / "plan.json") == old_replay["plan"]["sha256"]
            == (previous / "plan.sha256").read_text().strip() == previous_followup["initial_plan_sha256"],
            "historical plan identity differs")
    require(old_replay_sha256 == previous_followup["initial_replay_sha256"]
            and digest(previous / "followup-plan.json") == (previous / "followup-plan.sha256").read_text().strip(),
            "historical full replay differs from archived follow-up pins")
    matching_refs = ("policy", "timer", "as_run_inventory", "pinned_peers")
    for key in matching_refs:
        require(old_plan[key]["sha256"] == plan[key]["sha256"], f"historical {key} changes")
    require(old_plan["fixture_content_sha256"] == plan["fixture_content_sha256"], "historical fixture content changes")
    for key in ("environment_sha256", "contract_sha256"):
        require(old_plan["identities"][key] == plan["identities"][key], f"historical {key} changes")
    shared_71768 = binding_digests(plan, CONTROL)
    require(shared_71768 == binding_digests(old_plan, CANDIDATE), "historical 71,768-byte actor changes")
    for peer in PEERS:
        require(old_plan["identities"][peer] == plan["identities"][peer], f"historical {peer} changes")
    for filename in ("metadata.json", "runtime.json"):
        require(digest(previous / "identities" / CANDIDATE / filename) == shared_71768[filename],
                f"historical 71,768-byte actor {filename} differs")
    pack_identity = read_json(archive / "as-run" / "artifact-verification.json")

    evidence = Evidence(previous, Findings())
    old_reports = {record["order"]: record for record in old_replay["runs"]}
    new_reports = {record["order"]: record for record in current["runs"]}
    orders = {}
    timing_summaries = {}
    examined = set()
    for run in runs:
        order = run["order"]
        old_report = old_reports[order]
        old_receipt = old_replay["receipts"][old_report["id"]]
        new_receipt = current["receipts"][run["id"]]
        old_pins = evidence.json(old_receipt["original_pins"], f"{order} historical pins")
        new_pins = read_json(archive / new_receipt["original_pins"]["path"])
        require(old_pins is not None and old_pins | {"actors": new_pins["actors"]} == new_pins,
                f"{order} historical recorded protocol changes")
        for actor in (CANDIDATE, *PEERS):
            alias = CONTROL if actor == CANDIDATE else actor
            for field in ("binary_sha256", "metadata_sha256"):
                require(old_pins["actors"][actor].get(field) == new_pins["actors"][alias].get(field),
                        f"{order} historical {actor} {field} changes")
        old_complete = evidence.json(old_receipt["complete"], f"{order} historical completion")
        require(old_complete == read_json(archive / new_receipt["complete"]["path"]),
                f"{order} historical completion counts or flags change")
        old_rows = evidence.json(old_receipt["summary"], f"{order} historical summary")
        new_rows = read_json(archive / new_receipt["summary"]["path"])
        require(old_rows is not None
                and [row["case"] for row in old_rows] == [row["case"] for row in new_rows],
                f"{order} historical fixture order differs")
        old = {row["case"]: row for row in old_rows}
        new = {row["case"]: row for row in new_rows}
        timing_summaries[order] = (old, new)
        for before, after in zip(old_rows, new_rows, strict=True):
            require(before | {"median_seconds": after["median_seconds"], "paired_x_roc": after["paired_x_roc"],
                              "paired_x_libdeflate": after["paired_x_libdeflate"]} == after,
                    f"{before['case']} historical non-timing fields change")
        comparisons = {}
        for peer in PEERS:
            old_wins = point_wins(old, CANDIDATE, peer)
            control_wins = point_wins(new, CONTROL, peer)
            candidate_wins = point_wins(new, CANDIDATE, peer)
            lost = sorted(old_wins - control_wins)
            gained = sorted(control_wins - old_wins)
            ratios = f"paired_x_{peer}"
            changed = sorted(set(lost + gained) | (control_wins ^ candidate_wins))
            examined.update(changed)
            old_result = old_report["peers"][peer]
            new_result = new_reports[order]["peers"][peer]
            require(len(old_wins) == old_result["point_estimate_wins"][CANDIDATE]
                    and len(control_wins) == new_result["point_estimate_wins"][CONTROL]
                    and len(candidate_wins) == new_result["point_estimate_wins"][CANDIDATE],
                    f"{order} {peer} replay count differs from summaries")
            floor = policy["peer_win_floors"][peer]
            comparisons[peer] = {
                "absolute_named_case_floor": floor,
                "old_71768_wins": len(old_wins),
                "old_71768_precision_supported_wins": old_result["precision_supported_wins"][CANDIDATE],
                "new_control_and_candidate": new_result,
                "new_point_wins_above_floor": {CONTROL: len(control_wins) - floor, CANDIDATE: len(candidate_wins) - floor},
                "new_point_wins_without_precision": {
                    actor: precision_exceptions(run, policy, peer, actor, new_result) for actor in MECH
                },
                "same_71768_worker_losses_across_campaigns": lost,
                "same_71768_worker_gains_across_campaigns": gained,
                "changed_case_ratios": {
                    name: {"old_71768": old[name][ratios][CANDIDATE],
                           "new_71768": new[name][ratios][CONTROL],
                           "new_71192": new[name][ratios][CANDIDATE]} for name in changed
                },
            }
        orders[order] = {"peers": comparisons, "completed": old_complete}

    for run in runs:
        order = run["order"]
        receipt = old_replay["receipts"][old_reports[order]["id"]]
        old_rows, new_rows = timing_summaries[order]
        order_cases = {}
        for name in sorted(examined):
            raw = evidence.json(receipt["observations"][name], f"{order} historical {name} observations")
            require(raw is not None, f"{order} historical observations missing for {name}")
            count = orders[order]["completed"]["balanced_blocks_per_case"]
            blocks, calls, _, outputs = case_blocks(raw, fixtures[name], count)
            verify_summary(old_rows[name], fixtures[name], calls, blocks, outputs)
            order_cases[name] = timing_shift(blocks, old_rows[name], run, new_rows[name])
        orders[order]["paired_wall_time_diagnostic"] = order_cases

    return {
        "scope": "Descriptive comparison of two completed campaigns. All recorded blocks remain in the v1 verdict.",
        "historical_71768_actor": CANDIDATE,
        "source_71768_actor": CONTROL,
        "historical_plan_sha256": old_replay["plan"]["sha256"],
        "historical_full_replay_sha256": old_replay_sha256,
        "source_plan_sha256": current["plan"]["sha256"],
        "source_full_replay_sha256": digest(archive / "full-replay.json"),
        "artifact_verification": pack_identity,
        "matching_71768_object_worker_and_receipts": shared_71768,
        "matching_peers": {peer: plan["identities"][peer] for peer in PEERS},
        "matching_fixture_content_sha256": plan["fixture_content_sha256"],
        "matching_plan_reference_sha256": {key: plan[key]["sha256"] for key in matching_refs},
        "matching_environment_sha256": plan["identities"]["environment_sha256"],
        "matching_contract_sha256": plan["identities"]["contract_sha256"],
        "orders": orders,
    }
