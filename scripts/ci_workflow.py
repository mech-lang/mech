"""Structural YAML input for CI ownership, with no object constructors.

Scalars remain strings (not YAML 1.1 booleans such as the `on` key). Reject
duplicate/merge keys, custom tags and recursive aliases rather than assigning
execution ownership to an ambiguous or unsupported document.

Install the CI-only dependency in an isolated environment with
`python -m pip install --require-hashes -r .github/ci/python-requirements.txt`.
"""

from __future__ import annotations

import yaml


class WorkflowLoader(yaml.BaseLoader):
    def construct_object(self, node, deep=False):
        if node.tag not in {
            "tag:yaml.org,2002:str", "tag:yaml.org,2002:seq", "tag:yaml.org,2002:map",
            "tag:yaml.org,2002:bool", "tag:yaml.org,2002:int",
            "tag:yaml.org,2002:float", "tag:yaml.org,2002:null",
        }:
            raise yaml.YAMLError("unsupported workflow tag")
        return super().construct_object(node, deep)

    def construct_mapping(self, node, deep=False):
        keys = [self.construct_object(key, deep) for key, _ in node.value]
        if any(not isinstance(key, str) for key in keys):
            raise yaml.YAMLError("workflow keys must be scalars")
        if "<<" in keys or len(keys) != len(set(keys)):
            raise yaml.YAMLError("duplicate or merge workflow key")
        return super().construct_mapping(node, deep)


def workflow_document(source: str) -> dict:
    try:
        document = yaml.load(source, Loader=WorkflowLoader)
    except (yaml.YAMLError, RecursionError):
        return {}

    verified = set()

    def supported(value, ancestors, depth=0):
        if depth > 64 or id(value) in ancestors:
            return False
        if id(value) in verified:
            return True
        if isinstance(value, str):
            return True
        if not isinstance(value, (list, dict)):
            return False
        children = value.values() if isinstance(value, dict) else value
        valid = all(supported(child, (*ancestors, id(value)), depth + 1) for child in children)
        if valid:
            verified.add(id(value))
        return valid

    return document if isinstance(document, dict) and supported(document, ()) else {}


def step_contexts(source: str):
    """Yield only real step mappings with the complete enclosing job."""
    document = workflow_document(source)
    jobs = []
    if isinstance(document.get("steps"), list):
        jobs.append(document)  # isolated workflow-step fixtures
    runs = document.get("runs")
    if isinstance(runs, dict) and runs.get("using") == "composite":
        jobs.append(runs)
    mapping = document.get("jobs", {})
    if isinstance(mapping, dict):
        jobs.extend(job for job in mapping.values() if isinstance(job, dict))
    for job in jobs:
        steps = job.get("steps")
        if isinstance(steps, list):
            for step in steps:
                if isinstance(step, dict) and not {"run", "uses"}.issubset(step):
                    yield document, job, step
