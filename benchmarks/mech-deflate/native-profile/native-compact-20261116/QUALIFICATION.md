# Native compact qualification

Use `qualify.py` to evaluate a campaign for the Apple ARM64 diagnostic entry.
Record a campaign plan and keep its SHA-256 independently of the evidence
manifest before taking measurements. Preserve every planned run in the campaign
inventory, including adverse attempts. Each run keeps its own identifier and
raw observations.

```sh
python3 qualify.py campaign.json --evidence-root /path/to/evidence \
  --plan-sha256 "$CAMPAIGN_PLAN_SHA256" --promotion
```

The command returns a JSON decision and references to the evidence used.
Promotion exits 0 for **Retain**, 1 for **Reject**, and 2 for **Unresolved**.
Omit `--promotion` to inspect a historical campaign with incomplete provenance.
The decision in its output still applies.

Run `python3 qualify_archived.py --list` to see the committed experiments.
Supply a configuration name to replay its published observations, or add
`--manifest` to inspect the input references. This route uses raw files pinned
by the committed source and evidence ledger. The legacy campaigns retain their
original evidence, including missing provenance for a current promotion.
[Archived evaluation results](evidence/qualification-v1/archived-decisions.json)

## Acceptance policy v3

Use `policy_version: 3` in the manifest and the independently pinned plan
for new campaigns. Every complete order must record **81/81 against Roc**
and at least **62/81 against libdeflate**. Both orders must include the
historical natural 8 KiB libdeflate win and retain every peer win from the
corresponding control order. **63/81** is the next performance target.
Per-case slowdowns continue to be reported. Peer wins govern the timing
decision, along with the correctness and resource checks below.
[Policy v3](qualification-policy-v3.json)

## Acceptance policy v2

Replay v2 campaigns with `policy_version: 2` in their original manifests
and plans. Promotion requires both complete orders, at least
**81/81 Roc** and **61/81 libdeflate** point wins in each, and preservation of
every peer win from the corresponding control order. A gain on a different
case cannot cover a loss. Exact parity is a tie. The 74-contract view and
precision-supported win counts remain available in the report.

Candidate/control slowdowns and repeatability estimates are reported as
timing observations. Their confidence intervals use the same paired-block
method and 0.5% reporting threshold as v1. On a roughly 40 ns invocation,
0.5% is about 0.2 ns, making per-case timing changes sensitive to the timer
and linked layout even when the peer outcomes are reproducible. In v2,
the peer comparisons determine the runtime promotion gate. All fixture,
artifact, output, alias, error-origin, stack, resource, and improvement
checks apply. All predeclared evidence remains required and is replayed.

Evaluate historical manifests with their original `policy_version: 1`.
Each campaign retains its original observations and decision.
[Policy v2](qualification-policy-v2.json) · [Policy v1](qualification-policy-v1.json)

## Acceptance policy v1

The policy freezes the 81 named fixtures and their input, prefix, output, and
capacity contracts. Replay validates each complete order in the original and
reversed fixture directions. The secondary 74-contract count credits a win
when every equivalent named case wins. The 81-case floor uses the named cases.

Every complete order requires at least 81 Roc wins and 61 libdeflate wins by
point estimate. The candidate must keep each case its paired control wins.
Matching the control below a floor leaves the result unresolved. The report
counts precision-supported wins separately using a paired interval whose upper
bound is strictly below parity. Exact parity counts as a tie.

The per-case candidate/control policy uses a 90% paired-block bootstrap with
8,000 resamples stratified by run and actor order. An upper bound at or below
0.005 bounds the case at the declared resolution. A lower bound above 0.005
establishes a regression. An interval crossing the boundary requires more
evidence. The per-case intervals support decisions at the declared resolution.
Each interval describes an individual case.

The duplicate-control study measures two instances of the same control and
the candidate. A control interval extending outside ±0.005 leaves repeatability
unresolved. A study with stable controls and candidate bounds can resolve an
initially inconclusive case. The report keeps the initial and follow-up
intervals. An adverse individual interval conflicting with the combined bound
leaves the campaign unresolved and keeps the adverse run visible.

Replay checks timed-call output, capacity, inflation, aliases, and cleanup.
Failure probes must keep the expected status and semantic source origin.
Numeric origin indices can differ when each index resolves to the expected
source node and local origin in that artifact's metadata. Stack probes cover
the full suite and report the maximum observed extent for each subject.
Allocation counters describe the diagnostic entry's applicable resource
contract. Managed-entry admission and recovery require their own policy.

Retention also requires the predeclared improvement. Encoder text is measured
from the object. Complete build time includes final linking with compilers and
adapters already built. RSS records the compiler process campaign peak.
Runtime payload peak comes from timed calls. Runtime improvement requires a
paired interval below parity for at least one case. The report presents each
dimension, including root frame and measured stack, separately.

## Evidence manifest v1

Each evidence reference has `path` and `sha256`. Paths are relative to the
evidence root. Hashes identify the stored bytes, including the gzip container
for compressed JSON. A digest without a path records an archived identity and
leaves that file unverified. The evidence root contains the frozen
`fixtures.json` and its `fixture-data` files.

The manifest declares these fields.

| Field | Value |
| --- | --- |
| `schema_version` | 1 |
| `policy_version` | 3 for new campaigns, 1 or 2 for their historical replays |
| `objective` | One of `encoder_text_bytes`, `build_seconds`, `compiler_peak_rss_bytes`, `runtime_peak_payload_bytes`, or `runtime` |
| `environment` | Reference to JSON with `cpu`, `os`, and `clock`, plus the recorded conditions |
| `contract` | Reference to JSON with `scope` equal to `diagnostic`, `level` equal to 12, the frozen `fixture_inventory_sha256`, and the `timing_boundary` |
| `plan` | Reference to the independently pinned predeclared plan |
| `artifacts` | Provenance mapped by actor name |
| `runs` | Every full, focused, or repeated timed run |
| `duplicate_controls` | Every repeatability study |
| `checks` | Linked correctness calls, checked failures, and stack probes |
| `metrics` | Reference to the complete build trials |

The plan contains `policy_version`, `objective`, `environment_sha256`,
`contract_sha256`, the `worker_sha256` mapping for all four actors, and the
per-actor `artifact_sha256` identity mappings. Its
`runs` rows contain `id`, `kind` (`full` or `focus`), `order` (`forward`
or `reverse`), and `blocks_per_case`. It records `duplicate_control_ids`
and any prescribed `duplicate_control_cases`. The `build_orders` list
specifies the actor sequence for each timed complete build trial. An optional
`max_peak_stack_bytes` declares the supported stack ceiling.

The actor names are `mech-control`, `mech-experiment`, `roc`, and
`libdeflate`. Mech actor records include the full compiler `source_commit`,
references to `source`, `compiler`, `bytecode`, `object`, `metadata`,
`runtime`, `adapter`, `worker`, and `options`, and a `build_receipt`
reference. If compilation uses separate compiler executables, `compiler`
is the compiler inventory identifying those executables. The build receipt
records the source revision, the same identity mapping as `artifact_sha256`,
and invocation arrays under `commands.frontend`, `commands.emit`, and
`commands.link`. A separate `build_executable` reference records a distinct
source-to-link adapter when present. The peer actor records pin their workers,
with Roc metadata and any objects used as build metrics.

Each timing run points to `pins`, `summary`, `complete`, and a mapping from
case name to raw `observations`. Actual case order and balanced permutations
are validated. `cleanup` maps actors to their session footer logs. The pins
record the measured worker and metadata digests under `actors`, the
`environment_sha256` and `contract_sha256`, and the timed
`input_inventory_sha256`. Older absolute-path inventories can be supplied as
`fixture_inventory` and must agree with the frozen case identities. The pins
also identify level 12 and fresh libdeflate compression within timing.

A duplicate-control study row has `id`, `study`, `results`, and a case
mapping of raw `observations`. Its study receipt records the environment and
contract digests, three worker identities, the batch floor, repetitions, case
order, and deterministic seed. Replay recomputes the intervals from individual
durations. The timer can report a zero-length individual call when the call
falls within one clock tick. Each paired block requires positive means.

`checks.correctness` contains `receipt` and `calls` references. The receipt
ties the exact tested candidate worker and complete metadata to the per-case
output checks. Each correctness call supplies an `output_sha256` digest for
the raw compressed stream. `checks.checked_failures` refers to the paired raw failure
record. It adds `subjects` for the Mech actors, each containing the
`object_sha256`, `metadata_sha256`, and `probe_worker` reference. A stack
probe uses the same binding as its `subject` and keeps both observed samples
per case in `cases`. `checks.stack` supplies `control` and `candidate`
probe references.

The build metrics file uses `scope` equal to `source-through-final-link`,
`prebuilt_compilers_and_adapters` equal to true, the matching environment
digest, and a `timed_trials` list. Each trial declares its actor `order`,
elapsed `seconds`, `peak_rss_bytes`, and `outputs` by actor. Each output
records the actual `object_sha256` and `executable_sha256`. Keep warmups and
instrumented timing diagnostics outside the timed trials.

Previously measured runtime evidence remains eligible when the worker,
metadata, case contract, and pinned measurement conditions match. A newly
linked worker with a different digest needs runtime evidence tied to that
worker. The historical archives retain their original raw formats and
commands. Missing provenance produces Unresolved even when the recorded
point-estimate wins pass.

## Behavioral tests

```sh
python3 -B -m unittest discover -s tests -v
```
