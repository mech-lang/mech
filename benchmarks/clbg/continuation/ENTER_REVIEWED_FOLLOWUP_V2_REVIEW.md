# Exact completed-seal entry repair

This new mutable v2 entry preserves the frozen v1 entry and both failed job receipts. The unchanged successful Pony work-v5 seal completed normally at 2026-10-13T15:42:48.248499 UTC; the unchanged Prolog runtime-v4 seal completed normally at15:43:18.421825 UTC. Both v1 next-stage entries stopped at the worker-listing equality before any downstream command was reached. Their failed terminals and traces remain failed; their fixtures and successful seals are reused as historical evidence rather than rerun.

The launch records name `/opt/homebrew/opt/python@3.14/bin/python3.14`. Both authoritative `worker-started.json` and terminal worker records name `/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python`, with exactly identical worker identity records. The same PID, five start-time tokens and all remaining argv appear in launch and terminal. PPID changes to1. These are distinct executable files; no equality of their bytes or a general executable-path alias is claimed.

The primary [CPython v3.14.8 startup source](https://raw.githubusercontent.com/python/cpython/v3.14.8/Mac/Tools/pythonw.c) locates the app interpreter within the Python framework, replaces argv[0], and uses POSIX_SPAWN_SETEXEC. This supports the narrow interpretation of the saved transition. It does not qualify a benchmark runtime or replace any original runtime image.

Before launch, root must create and review a concrete `--host-linkage` JSON under the common mutex and pass its actual SHA through `--host-linkage-sha256`. Its required fields are:

```json
{
  "status": "root-reviewed-exact-python-framework-linkage",
  "python_version": "3.14.8",
  "official_startup_source": "https://raw.githubusercontent.com/python/cpython/v3.14.8/Mac/Tools/pythonw.c",
  "same_image_bytes_claimed": false,
  "benchmark_runtime_image_changed": false,
  "launcher": {
    "path": "/opt/homebrew/opt/python@3.14/bin/python3.14",
    "resolved_path": "/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/bin/python3.14",
    "sha256": "e401ac7a95cda75dee34a0cfe1e4faf0a6df1a501cf67887f177fb78cd4eb60a"
  },
  "framework": {
    "path": "/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python",
    "sha256": "ACTUAL_ROOT_REVIEWED_CURRENT_FRAMEWORK_IMAGE_SHA_REQUIRED"
  }
}
```

The displayed framework placeholder is intentionally not a valid digest or ready descriptor. Its current digest must be captured under common; this fresh host pin is not claimed to be an independently captured historical worker-image digest. The bin launcher digest is retained from actual qualified host receipts. V2 rechecks both actual images and the exact launcher symlink resolution, pins their distinct bytes, requires the exact two recorded image paths, compares the actual worker-started identity with terminal without discarding argv[0], and checks all remaining launch argv and birth tokens. An arbitrary changed image, different PID/birth/argv or different post-startup worker fails.

All target command identities, actual seal statuses, fixture inventories, original source/binary/runtime pins and downstream prerequisite validators are copied unchanged from v1. Prolog still requires the exact compatible runtime-v4/v2 published bytes, original six fixture closure and bundled Python host. Pony still requires the preserved four actual Pi/K work traces,14 fixtures,20 source-work profiles and exactly two new tree observations; full timings remain unauthorized. Successful v2 entry keeps the same durable owner through exec into the unchanged target, after releasing its review descriptor so that target acquires common itself.

The new exclusive metadata journals are `jobs/pony-work-reviewed-next-stage-v2` and `jobs/prolog-runtime-reviewed-next-stage-v2`; the proposed durable job names are `pony-two-tree-reviewed-next-stage-v2` and `prolog-private-runtime-reviewed-next-stage-v2`. No output from v1 is overwritten or resumed. The v2 journal carries the exact old failed trace, failed terminal and stopped-before-stage receipt, the completed dependency records, new linkage descriptor and exact transition disclosure.

Source authoring/read only. No v2 import, hash, fixture, native command, freeze or launch has run. Root and independent peer must read the complete new v2 source and accept the actual host linkage before launch.

Root subsequently completed its full source read and captured the concrete descriptor under common at `jobs/python-framework-host-linkage-v1/host-linkage.json`, SHA `9a1c0fa4fc330cf0c7c176349c220640ad4714c5e8e081eaead84c00b89729af`. Its framework image SHA is `b3e2d3162eb789101425bbe76810fc5eb0d412d7cd5867d27568dd180ccf5a22`. Root and the independent C3 reviewer both accepted the complete v2 source and concrete descriptor. Parent authorized one fresh freeze/entry for each stage after that acceptance; actual freeze and launches are recorded separately rather than inferred from this source review.
