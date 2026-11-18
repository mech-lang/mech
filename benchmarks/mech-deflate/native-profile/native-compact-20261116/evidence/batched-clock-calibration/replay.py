"""Verify the archived batch-clock evidence and replay its exact planned cohorts."""

import argparse
from hashlib import file_digest
import importlib.util
import json
from math import isclose
from pathlib import Path
import shutil
import tarfile
from tempfile import TemporaryDirectory

ACTORS = ("control", "duplicate-control", "candidate")
HISTORICAL_ACTORS = {"control": "mech-control", "candidate": "mech-experiment"}
CLEANUP = "CLEANUP private_payload_bytes=0 live_owner_records=0"
SOURCES = ("batch_worker.rs", "campaign.py", "pinned_allocator.rs", "qualification_records.py",
           "qualification_timings.py", "qualification-policy-v1.json")


def read(path):
    return json.loads(path.read_text())


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def members():
    names = {"plan.json", "plan.sha256", "environment.json", "contract.json", "fixtures.json"}
    names.update(f"source/{name}" for name in SOURCES)
    names.update(f"{actor}/{name}" for actor in ACTORS for name in ("batch-worker", "metadata.json", "program.o"))
    names.update(f"{actor}/build.log" for actor in HISTORICAL_ACTORS)
    names.update(f"{area}/{name}" for area in ("acquisition", "smoke")
                 for name in ("complete.json", "receipt.json", "observations.jsonl", "warmups.jsonl"))
    names.update(f"{area}/{actor}.stderr" for area in ("acquisition", "smoke") for actor in ACTORS)
    names.add("acquisition/replay.json")
    names.update(f"logs/{name}.log" for name in ("setup", "checks", "acquire", "replay"))
    return names


def unpack(bundle, study):
    remaining = members()
    with tarfile.open(bundle, mode="r|gz") as archive:
        for item in archive:
            assert item.isfile() and item.name in remaining, item.name
            remaining.remove(item.name)
            target = study / item.name
            target.parent.mkdir(exist_ok=True, parents=True)
            with archive.extractfile(item) as source, target.open("xb") as output:
                shutil.copyfileobj(source, output)
    assert not remaining, sorted(remaining)


def verify_historical(packet, plan, contract):
    old = packet / "evidence/bound-71768-runtime"
    old_plan_file = old / "plan.json"
    assert digest(old_plan_file) == plan["previous_plan"]["sha256"]
    assert digest(old_plan_file) == (old / "plan.sha256").read_text().strip()
    assert digest(old / "full-and-repeat-replay.json") == contract["previous_receipt"]["sha256"]
    assert read(old / "full-and-repeat-replay.json")["decision"] == "Unresolved"
    assert digest(old / "clock-quantization.json") == contract["previous_clock_quantization"]["sha256"]
    assert read(old / "contract.json") == contract["underlying_contract"]
    assert digest(old / "contract.json") == read(old_plan_file)["identities"]["contract_sha256"]
    for actor, old_label in HISTORICAL_ACTORS.items():
        legacy = plan["actors"][actor]["legacy"]
        bindings = read(old_plan_file)["bindings"][old_label]
        assert {name: item["sha256"] for name, item in legacy.items()} == {
            name: item["sha256"] for name, item in bindings.items()
        }
        assert plan["actors"][actor]["object"]["sha256"] == legacy["program.o"]["sha256"]
        assert plan["actors"][actor]["metadata"]["sha256"] == legacy["metadata.json"]["sha256"]
        assert plan["actors"][actor]["worker"]["sha256"] != legacy["mech-worker"]["sha256"]


def verify_artifacts(study, plan):
    for actor, recorded in plan["actors"].items():
        for kind, name in (("object", "program.o"), ("metadata", "metadata.json"), ("worker", "batch-worker")):
            assert digest(study / actor / name) == recorded[kind]["sha256"], (actor, kind)
    for kind in ("object", "metadata", "worker"):
        a, b, c = [plan["actors"][actor][kind]["sha256"] for actor in ACTORS]
        assert a == b and a != c, kind

    checked = {}
    for area in ("acquisition", "smoke"):
        output = study / area
        receipt = read(output / "receipt.json")
        done = read(output / "complete.json")
        assert receipt["plan_sha256"] == done["plan_sha256"] == digest(study / "plan.json")
        assert receipt["protocol"] == plan["protocol"] and receipt["mode"] == done["mode"]
        assert len({tuple(inode) for inode in receipt["worker_inodes"]}) == 3
        checked[area] = done["mode"]
        for file in ("observations", "warmups"):
            assert digest(output / f"{file}.jsonl") == done[file]["sha256"]
        for actor in ACTORS:
            log = output / f"{actor}.stderr"
            assert digest(log) == done["cleanup"][actor]["sha256"]
            assert log.read_text().strip().endswith(CLEANUP)
    assert checked == {"acquisition": "clock-observations", "smoke": "all-81-call-checks"}


def verify_fixtures(study, packet, campaign, policy):
    canonical = campaign.fixture_inventory(packet, policy)
    copied = read(study / "fixtures.json")
    assert len(canonical) == len(copied) == 81
    data_fields = {"input", "prefix", "expected"}
    for old, live in zip(copied, canonical, strict=True):
        assert {key: value for key, value in old.items() if key not in data_fields} == {
            key: value for key, value in live.items() if key not in data_fields
        }
        for field in data_fields:
            assert Path(old[field]).name == Path(live[field]).name
    smoke = study / "smoke"
    done = read(smoke / "complete.json")
    assert done["runs"] == len(copied) and (smoke / "warmups.jsonl").stat().st_size == 0
    with (smoke / "observations.jsonl").open() as rows:
        for index, fixture in enumerate(copied):
            for actor in ACTORS:
                record = json.loads(next(rows))
                assert record["run"] == index and record["actor"] == actor
                campaign.validate(json.loads(record["reply"]), fixture, 0, 1, 1)
        assert not rows.readline()


def compare(expected, replayed):
    assert type(expected) is type(replayed)
    if isinstance(expected, dict):
        assert expected.keys() == replayed.keys()
        for key in expected:
            compare(expected[key], replayed[key])
    elif isinstance(expected, list):
        for old, fresh in zip(expected, replayed, strict=True):
            compare(old, fresh)
    elif isinstance(expected, float):
        assert isclose(expected, replayed, rel_tol=1e-11, abs_tol=1e-13)
    else:
        assert expected == replayed


def replay(evidence, packet):
    bundle = evidence / "clock-study.tar.gz"
    assert digest(bundle) == (evidence / "clock-study.sha256").read_text().strip()
    with TemporaryDirectory(prefix="mech-batch-replay-") as scratch:
        study = Path(scratch)
        unpack(bundle, study)
        for file, item in (("plan.json", "plan.json"), ("plan.sha256", "plan.sha256"),
                           ("acquisition/replay.json", "report.json"),
                           ("environment.json", "environment.json"), ("contract.json", "contract.json")):
            assert digest(study / file) == digest(evidence / item)
        plan = read(study / "plan.json")
        assert digest(study / "plan.json") == (study / "plan.sha256").read_text().strip()
        for name in SOURCES:
            assert digest(study / "source" / name) == plan["sources"][name]["sha256"]
        assert plan["scope"] == "calibration"
        assert plan["epochs"] == 2 and plan["groups"] == [1, 256]
        assert plan["blocks_per_order"] == 32 and plan["calls_per_actor_block"] == 2048
        assert plan["cases"] == ["empty", "one-stored", "seven-stored-prefix", "random-64"]
        policy_file = study / "source/qualification-policy-v1.json"
        assert digest(policy_file) == digest(packet / policy_file.name)
        policy = read(policy_file)
        assert (policy["runtime_resolution"], policy["bootstrap_coverage"], policy["bootstrap_resamples"]) == (0.005, 0.90, 8000)

        verify_historical(packet, plan, read(study / "contract.json"))
        verify_artifacts(study, plan)

        module_spec = importlib.util.spec_from_file_location("recorded_batch_campaign", study / "source/campaign.py")
        campaign = importlib.util.module_from_spec(module_spec)
        module_spec.loader.exec_module(campaign)
        verify_fixtures(study, packet, campaign, policy)

        result_file = study / "acquisition/replay.json"
        recorded = read(result_file)
        result_file.unlink()
        campaign.replay(study)
        compare(recorded, read(result_file))
        print("Verified all four cases, both epochs, 243 smoke calls and the original Unresolved decision.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packet", type=Path, help="root of the canonical benchmark packet")
    args = parser.parse_args()
    evidence = Path(__file__).resolve().parent
    packet = args.packet.resolve() if args.packet else evidence.parents[1]
    replay(evidence, packet)
