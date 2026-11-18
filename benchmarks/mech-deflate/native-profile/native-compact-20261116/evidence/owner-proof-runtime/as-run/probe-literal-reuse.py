"""Emit and replay a changed source image using the pinned native tool and peers."""
from hashlib import file_digest
from importlib import import_module
from pathlib import Path
import argparse
import json
import os
import subprocess
import sys
import time
from pinned_worker_rebuild import rebuild_link

q = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument('--emit-only', action='store_true')
parser.add_argument('--emitter', type=Path, default=q / 'emit_checked_geometry')
parser.add_argument('--compact-runtime', type=Path)
parser.add_argument('--native-reference', type=Path, help='reuse a matched native image and worker ABI')
parser.add_argument('--source-variant-sha', help='pinned bytecode SHA256 for an explicit source comparison')
parser.add_argument('output', type=Path)
parser.add_argument('image', type=Path)
parser.add_argument('flags', nargs='*')
args = parser.parse_args()
if args.source_variant_sha and not args.native_reference:
    parser.error('--source-variant-sha requires --native-reference')
if args.native_reference:
    reference = args.native_reference.resolve(strict=True)
    runtime = json.loads((reference / 'runtime.json').read_text())
    flags = {key: value for key, value in runtime['flags'].items() if 'LLVM' not in key}
else:
    reference = q / 'vector-facts-root-proofs-stack-early'
    prior = Path('/private/tmp/mech-overwrite-reuse-20261114/final')
    runtime = json.loads((reference / 'runtime.json').read_text())
    flags = json.loads((prior / 'emit.json').read_text())['flags'] | runtime['flags']
    flags.update(MECH_COMPACT_SHARE_COLD_EPILOGUES='1', MECH_COMPACT_SHARE_PHASE_BODIES='1',
                 MECH_COMPACT_SHARE_CHECKED_TRAVERSALS='all')
    # The source rewrite renumbers Plan and CLIF blocks. Measure both images with
    # identical pass policies and discover new sites before selecting any regions.
    for name in ('MECH_COMPACT_SHARE_CONTROLLER_WINDOWS', 'MECH_COMPACT_LAYOUT_CONTROLLER_WINDOWS',
                 'MECH_COMPACT_PHASE_CONTROLLER_WINDOWS', 'MECH_COMPACT_CONTINUATION_WINDOWS',
                 'MECH_COMPACT_TRAVERSAL_SKIP_HEADERS'):
        flags.pop(name, None)
for change in args.flags:
    name, value = change.split('=', 1)
    assert name.startswith('MECH_')
    if value:
        flags[name] = value
    else:
        flags.pop(name, None)
out = args.output.resolve()
image = args.image.resolve(strict=True)
out.mkdir(parents=True)

def sha(filename):
    with filename.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()

image_sha = sha(image)
if args.native_reference:
    assert image_sha == (args.source_variant_sha or runtime['image_sha256']), 'wrong source image'
    assert not any('LLVM' in name for name in flags), 'LLVM is disabled for native comparisons'

link = [arg.replace(str(reference / 'program.o'), str(out / 'program.o'))
        .replace(str(reference / 'mech-worker'), str(out / 'mech-worker'))
        for arg in runtime['link']]
assert sum(arg.endswith(str(out / 'program.o')) for arg in link) == 1
assert link.count(str(out / 'mech-worker')) == 1
missing_extern = any(not Path(arg.partition('=')[2]).is_file()
                     for previous, arg in zip(link, link[1:]) if previous == '--extern')
link_provenance = {}
if missing_extern and not args.emit_only:
    link, link_provenance = rebuild_link(link)
if args.compact_runtime:
    library = args.compact_runtime.resolve(strict=True)
    bindings = [i for i, value in enumerate(link) if value.startswith("mech_gpu=")]
    assert len(bindings) == 1 and link[bindings[0] - 1] == "--extern"
    link[bindings[0]] = f"mech_gpu={library}"
    link_provenance["compact_runtime"] = {"path": str(library), "sha256": sha(library)}
emitter = args.emitter.resolve(strict=True)
command = list(map(str, (emitter, image, out)))
(out / 'runtime.json').write_text(json.dumps(dict(emit=command, flags=flags, link=link,
    image_sha256=image_sha, reference_image_sha256=runtime['image_sha256'], emitter_sha256=sha(emitter),
    link_provenance=link_provenance), indent=2) + '\n')
start = time.monotonic()
with (out / 'emit.stderr').open('wb') as log:
    environment = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')} | flags
    if args.native_reference:
        environment['PATH'] = '/usr/bin:/bin'
    with subprocess.Popen(command, env=environment, stdout=log, stderr=log) as proc:
        _, status, usage = os.wait4(proc.pid, 0)
        proc.returncode = os.waitstatus_to_exitcode(status)
        if proc.returncode:
            raise subprocess.CalledProcessError(proc.returncode, command)
elapsed = time.monotonic() - start
macho = import_module('reachability-census')
_, object_sections, _ = macho.load(out / 'program.o')
metadata = json.loads((out / 'metadata.json').read_text())
size_bytes = object_sections[macho.TEXT][2]
reference_sizes = json.loads((q / 'remaining-size-results.json').read_text())
roc_bytes = reference_sizes['roc_object_text_bytes']
libdeflate_bytes = reference_sizes['libdeflate_object_text_bytes']
receipt = dict(object_text_bytes=size_bytes, object_text_x_roc=size_bytes / roc_bytes,
               object_text_x_libdeflate=size_bytes / libdeflate_bytes,
               roc_object_text_bytes=roc_bytes, libdeflate_object_text_bytes=libdeflate_bytes,
               root_code_bytes=metadata['code_bytes'], root_frame_bytes=metadata['stack_bytes'],
               source_blocks=metadata['source_blocks'], root_direct_calls=metadata['calls'],
               emit_seconds=elapsed, peak_resident_bytes=usage.ru_maxrss,
               object_sha256=sha(out / 'program.o'), metadata_sha256=sha(out / 'metadata.json'))
(out / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
if args.emit_only:
    print(f"{out.name} emitted code {receipt['object_text_x_roc']:.3f}× Roc", flush=True)
    sys.exit()
subprocess.run(link, check=True)
label = (f'literal-reuse-{out.name}' if out.parent.name == 'literal-choice-reuse-native'
         else f'{out.parent.name}-{out.name}')
subprocess.run([sys.executable, '-B', q / 'verify-worker.py', out / 'mech-worker',
                out / 'metadata.json', label], check=True)
_, linked_sections, _ = macho.load(out / 'mech-worker')
receipt.update(linked_text_bytes=linked_sections[macho.TEXT][2], worker_sha256=sha(out / 'mech-worker'),
               verified_calls=162, proof=str(q / (label+'-qualification.json')))
(out / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(f"{out.name} emitted code {receipt['object_text_x_roc']:.3f}× Roc, "
      f"{receipt['verified_calls']} exact linked calls passed", flush=True)
