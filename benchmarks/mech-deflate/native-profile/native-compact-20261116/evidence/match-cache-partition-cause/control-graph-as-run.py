"""Inspect the archived Mech v1 control graph and bind compact origins to it."""

import json
from pathlib import Path
from struct import unpack_from


Q = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification")
STUDY = Q / "match-cache-offset-slot"
ROLES = ("control", "stored-table-slot", "stored-arithmetic-slot")


def sections(path):
    data = path.read_bytes()
    magic, version, header_size = unpack_from("<4sHH", data)
    assert (magic, version, header_size) == (b"MECH", 1, 64)
    count, = unpack_from("<H", data, 24)
    assert count in (18, 19)
    result = {}
    for index in range(count):
        kind, flags, items, offset, length, reserved = unpack_from("<HHIQQQ", data, 64 + 32 * index)
        assert kind == index + 1 and not reserved and offset + length <= len(data)
        if kind in (8, 9, 13, 17):
            result[kind] = json.loads(data[offset:offset + length])
    return result


def blocks_in(item, ancestry=()):
    if isinstance(item, dict):
        if {"id", "parameters", "operations", "yield_value"} <= item.keys():
            yield item, ancestry
            for op in item["operations"]:
                parent = (item["id"], op["node"], next(iter(op["body"])) if isinstance(op["body"], dict) else op["body"])
                yield from blocks_in(op["body"], (*ancestry, parent))
        else:
            for value in item.values():
                yield from blocks_in(value, ancestry)
    elif isinstance(item, list):
        for value in item:
            yield from blocks_in(value, ancestry)


def graph(image):
    block_pairs = list(blocks_in(image[13]["nodes"]))
    selected = {block["id"]: (block, parents) for block, parents in block_pairs}
    assert len(selected) == len(block_pairs)
    return selected


def operation(blocks, block_index, local):
    block, _ = blocks[block_index]
    item = block["operations"][local]
    assert item["node"] == local
    return item


def operation_name(item, names):
    body = item["body"]
    if isinstance(body, dict) and "Operation" in body:
        binding = names[body["Operation"]["operation"]]
        return "/".join([*binding["module_path"], binding["operation_name"]])
    return next(iter(body)) if isinstance(body, dict) else body


def argument(blocks, value, visited=frozenset()):
    """Trace captures through lazy branches and iteration inputs to their wire origin."""
    if "Parameter" not in value:
        return value
    parameter = value["Parameter"]
    block_number = parameter["block"]
    ordinal = parameter["ordinal"]
    key = (block_number, ordinal)
    assert key not in visited
    block, ancestry = blocks[block_number]
    source, _ = block["parameters"][ordinal]
    if not isinstance(source, dict) or "Capture" not in source or not ancestry:
        return value | {"parameter_source": source}
    parent_block, parent_local, kind = ancestry[-1]
    parent = operation(blocks, parent_block, parent_local)
    if kind == "Match":
        input_index = parent["body"][kind]["captures"][source["Capture"]][0]
    else:
        return value | {"parameter_source": source, "capture_parent": [parent_block, parent_local, kind]}
    return argument(blocks, parent["inputs"][input_index], visited | {key})


def summarize_graph(role):
    folder = STUDY / role
    image = sections(folder / "program.mecb")
    blocks = graph(image)
    metadata = json.loads((folder / "metadata.json").read_text())
    source = (folder / "source.mec").read_text()
    slot_decl = next(line for line in source.splitlines() if line.startswith("deflate-table-offset-slots :="))
    table = [int(value.removesuffix("u8")) for value in slot_decl.split("[", 1)[1][:-1].split()]
    slot_constant, = [index for index, item in enumerate(image[9]) if item["data"] == {
        "Matrix": [{"U8": byte} for byte in table]
    }]
    output = {"offset_slot_constant": slot_constant, "phases": [], "slot_uses": []}
    phases = {p["header"]: p for p in metadata["numeric_phases"]}
    annotated = {**{header: "outlined" for header in phases}}
    for phase in phases.values():
        for header in phase.get("private_fixed_nested", {}).get("loops", [])[1:]:
            annotated[header] = f"nested in {phase['header']}"
    annotated.setdefault(475, "no outlined phase")
    for header, role_name in sorted(annotated.items()):
        origin = metadata["origins"][header]
        block, local = origin["block"], origin["local"]
        item = operation(blocks, block, local)
        name = operation_name(item, image[17])
        body = item["body"].get(name, {}) if isinstance(item["body"], dict) else {}
        phase = {"header": header, "classification": role_name, "origin": [block, local],
                 "ancestry": blocks[block][1], "operation": name, "schema": item["schema"],
                 "input_count": len(item["inputs"]), "wire_details": {
                     k: v for k, v in body.items() if k in ("carrier", "seed", "captures", "kind")
                 }}
        if local and item["inputs"] == [{"Local": {"block": block, "node": local - 1}}]:
            seed = operation(blocks, block, local - 1)
            schema = image[8][seed["schema"]]["body"]
            if operation_name(seed, image[17]) == "core/composite-pack" and "Tuple" in schema:
                operands = seed["inputs"]
                phase["seed"] = {"origin": [block, local - 1], "fields": len(operands), "matrices": [
                    {"field": index, "type": field["Matrix"], "operand": value}
                    for index, (field, value) in enumerate(zip(schema["Tuple"], operands, strict=True))
                    if isinstance(field, dict) and "Matrix" in field
                ]}
        output["phases"].append(phase)
    for block, (item, ancestry) in blocks.items():
        for op in item["operations"]:
            for index, value in enumerate(op["inputs"]):
                ultimate = argument(blocks, value)
                if ultimate == {"Constant": slot_constant}:
                    output["slot_uses"].append({"at": [block, op["node"], index], "ancestry": ancestry,
                                                "operation": operation_name(op, image[17]), "wire_operand": value})
    return output


if __name__ == "__main__":
    print(json.dumps({role: summarize_graph(role) for role in ROLES}, indent=2))
