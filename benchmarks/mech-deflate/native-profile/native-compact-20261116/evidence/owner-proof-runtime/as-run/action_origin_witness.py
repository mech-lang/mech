"""Match a byte-append failure when an earlier scalar changes its local number."""
from functools import cache
from hashlib import file_digest
from pathlib import Path
import argparse
import json


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


@cache
def load_plan(actor):
    runtime = json.loads((actor / "runtime.json").read_text())
    image = Path(runtime["emit"][1]).resolve(strict=True)
    assert digest(image) == runtime["image_sha256"]
    proof = image.parents[1] / "lowered-actions.json"
    blocks = json.loads(proof.read_text())
    metadata = json.loads((actor / "metadata.json").read_text())
    assert len(blocks) == len(metadata["origins"])
    assert all(row["origin"] == origin for row, origin in zip(blocks, metadata["origins"], strict=True))
    return blocks, {"image_sha256": runtime["image_sha256"], "lowered_actions_sha256": digest(proof)}


def selected_calls(plan, origin):
    return [(index, row["action"]["LibraryCall"]["template"])
            for index, row in enumerate(plan)
            if row["origin"]["node"] == origin["node"]
            and row["origin"]["block"] == origin["block"] and "LibraryCall" in row["action"]]


def schema_independent_contract(contract):
    if isinstance(contract, list):
        return [schema_independent_contract(item) for item in contract]
    if isinstance(contract, dict):
        return {key: schema_independent_contract(value) for key, value in contract.items() if key != "schema"}
    return contract


def operands(template):
    lexical = template["selected"]["Lexical"]
    origin = template["origin"]
    assert lexical["block"] == origin["block"] and lexical["local"] == origin["local"]
    result = []
    for source in lexical["sources"]:
        kind, field = next(iter(source["Control"].items()))
        assert field["block"] == origin["block"]
        if kind == "Parameter":
            result.append([kind, field["ordinal"]])
        else:
            assert kind == "Local" and field["node"] < origin["local"]
            result.append([kind, origin["local"] - field["node"]])
    return result


def scalar_neighbors(plan, site):
    except_center = (site - 2, site - 1, site + 1, site + 2)
    return [plan[index]["action"]["Scalar"]["Primitive"]["op"] for index in except_center]


def compare_append(control, candidate, old_site, candidate_anchor):
    old_plan, old_pin = load_plan(control)
    new_plan, new_pin = load_plan(candidate)
    old_origin = old_plan[old_site]["origin"]
    new_block = new_plan[candidate_anchor]["origin"]
    old_calls, new_calls = selected_calls(old_plan, old_origin), selected_calls(new_plan, new_block)
    assert old_origin["node"] == new_block["node"]
    assert [t["selected_key"] for _, t in old_calls] == [t["selected_key"] for _, t in new_calls]
    ordinal, = (at for at, (site, _) in enumerate(old_calls) if site == old_site)
    _, before = old_calls[ordinal]
    new_site, after = new_calls[ordinal]
    assert before["selected_key"] == {"module_path": ["sequence"], "operation_name": "append-le-bytes"}
    assert before["inputs"] == after["inputs"] and before["output"] == after["output"]
    assert schema_independent_contract(before["contract"]) == schema_independent_contract(after["contract"])
    first, second = (t["selected"]["Lexical"]["leaf"] for t in (before, after))
    for key in ("primitive", "inputs", "output", "implementation_memory"):
        assert first[key] == second[key]
    roles = operands(before)
    assert roles == operands(after)
    neighbors = scalar_neighbors(old_plan, old_site)
    assert neighbors == scalar_neighbors(new_plan, new_site)
    return {
        "candidate_site": new_site,
        "candidate_anchor": candidate_anchor,
        "operation": "sequence/append-le-bytes",
        "ordinal_in_lexical_block": ordinal,
        "operand_roles": roles,
        "neighboring_scalar_operations": neighbors,
        "plan_pins": [old_pin, new_pin],
    }


def check_witness(report, control, candidate, before, after):
    witness = report.get("checked_action_witnesses", {}).get(str(before))
    if witness is None:
        return False
    actual = compare_append(control, candidate, before, witness["candidate_anchor"])
    assert actual == witness and actual["candidate_site"] == after, (actual, before, after)
    return True


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("control", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("source_map", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("sites", nargs="+", type=int)
    args = parser.parse_args()
    report = json.loads(args.source_map.read_text())
    witnesses = report.setdefault("checked_action_witnesses", {})
    for site in args.sites:
        anchor = report["source_mapping"][str(site)]["site"]
        witnesses[str(site)] = compare_append(args.control, args.candidate, site, anchor)
        print(site, witnesses[str(site)]["candidate_site"], "matched typed append")
    with args.output.open("x") as stream:
        json.dump(report, stream, indent=2)
        stream.write("\n")
