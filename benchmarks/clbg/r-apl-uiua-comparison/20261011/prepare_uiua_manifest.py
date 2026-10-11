#!/usr/bin/env python3
"""Pin existing Uiua sources and evidence without executing benchmark programs."""
from pathlib import Path
import hashlib
import json
import platform
import re

G = Path(__file__).resolve().parent
H = G / '.build/toolchains/uiua-0.19.1'
R = H / 'uiua'
U = H / 'uiua-0.19.1'
E = H / 'ecow-0.2.4'
PY = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def pins(paths):
    return {str(path): sha(path) for path in paths}


def loc(source):
    # Preserve physical noncomment lines. Expand independent assigned bindings;
    # a tacit pipeline, bracket branch, array literal, or data declaration is
    # one expression rather than several independent source statements.
    cleaned = []
    for line in source.splitlines():
        quoted = False
        escaped = False
        chars = []
        for char in line:
            if quoted:
                if escaped:
                    escaped = False
                elif char == '\\':
                    escaped = True
                elif char == '"':
                    quoted = False
                chars.append(' ')
            elif char == '"':
                quoted = True
                chars.append('S')
            elif char == '#':
                break
            else:
                chars.append(char)
        cleaned.append(''.join(chars))
    physical = sum(bool(line.strip()) for line in cleaned)
    expansions = []
    for number, line in enumerate(cleaned, 1):
        count = len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line))
        if count > 1:
            expansions.append({'line': number, 'extra_lines': count - 1,
                               'reason': 'Independent assigned bindings packed on one source line.'})
    return physical + sum(row['extra_lines'] for row in expansions), physical, expansions


def proof(path, first, last, conclusion):
    lines = path.read_text().splitlines()
    return {'path': str(path), 'sha256': sha(path), 'first_line': first,
            'last_line': last, 'excerpt': '\n'.join(lines[first - 1:last]),
            'conclusion': conclusion}


release = json.loads((H / 'release.json').read_text())
assert release['tag_name'] == '0.19.1'
asset = next(row for row in release['assets']
             if row['name'] == 'uiua-bin-aarch64-apple-darwin.zip')
assert sha(H / 'runtime.zip') == asset['digest'].removeprefix('sha256:')
assert sha(H / 'ecow-0.2.4.crate') == 'ef5eeffa816451f3a6a4cce9cd796e3e5ba6018638d3ce5cc7f87b73bababf60'
runtime = {
    'path': str(R), 'sha256': sha(R), 'version': 'Uiua 0.19.1',
    'architecture': 'aarch64-apple-darwin',
    'distribution_url': asset['browser_download_url'],
    'distribution_sha256': sha(H / 'runtime.zip'),
    'provenance': 'Official Uiua ARM64 macOS release binary; not a locally compiled substitute.',
    'source_url': 'https://github.com/uiua-lang/uiua/archive/refs/tags/0.19.1.tar.gz',
    'source_archive_sha256': sha(H / 'source.tar.gz'),
    'release_metadata': str(H / 'release.json'),
    'primary_runtime_documentation': 'https://www.uiua.org/docs',
    'pins': pins([R, H / 'runtime.zip', H / 'source.tar.gz', H / 'release.json',
                  U / 'Cargo.lock', H / 'ecow-0.2.4.crate']),
    'host_os': platform.mac_ver()[0],
    'system_library_policy': 'macOS platform libraries are supplied by the recorded host OS; no additional non-system runtime dylib is linked by Uiua.'
}
gmp = {
    'name': 'GMP', 'version': '6.3.0',
    'kind': 'Ordinary general-purpose arbitrary-precision integer library with a generic FFI adapter.',
    'package_url': 'https://gmplib.org/',
    'installed_library': '/opt/homebrew/opt/gmp/lib/libgmp.10.dylib',
    'support_source': str(G / 'support/uiua/gmp.c'),
    'support_build_command': ['cc', '-O3', '-fPIC', '-dynamiclib', str(G / 'support/uiua/gmp.c'), '-I/opt/homebrew/opt/gmp/include', '-L/opt/homebrew/opt/gmp/lib', '-lgmp', '-o', str(G / '.build/uiua-support/libgmp-uiua.dylib')],
    'support_build_command_status': 'Reconstruction recipe recorded; metadata finalizer did not invoke a compiler.',
    'support_library': str(G / '.build/uiua-support/libgmp-uiua.dylib'),
    'pins': pins([G / 'support/uiua/gmp.c',
                  G / '.build/uiua-support/libgmp-uiua.dylib',
                  Path('/opt/homebrew/opt/gmp/lib/libgmp.10.dylib'),
                  Path('/opt/homebrew/opt/gmp/include/gmp.h'),
                  Path('/opt/homebrew/opt/gmp/INSTALL_RECEIPT.json')]),
    'exported_operations': ['new', 'free', 'set', 'copy', 'add', 'sub', 'mul',
                            'mul-small', 'floor-div', 'compare', 'get'],
    'benchmark_specific_kernel': False,
    'division_audit': 'ui_big_floor_div is exactly a generic mpz_fdiv_q call. ExtractThree and ExtractFour each invoke Div separately for every Candidates attempt; spigot control, state updates, acceptance and output remain counted Uiua source.'
}

small_path = G / 'results/uiua/small-validation.json'
adversarial_path = G / 'results/uiua/adversarial-validation.json'
small = json.loads(small_path.read_text())
adversarial = json.loads(adversarial_path.read_text())
assert len(small) == 20 and all(row['reference_match'] for row in small.values())
assert len(adversarial) >= 80 and all(row['reference_match'] for row in adversarial)

tree_proof = {
    'method': 'Pinned runtime source proof; no substitute tree kernel or benchmark instrumentation.',
    'status': 'passed-source-audit',
    'fresh_node_allocation': True,
    'fresh_leaf_allocation': True,
    'retained_long_lived_tree': True,
    'proof_scope': 'Each constructor produces a new nonempty three-element boxed-node buffer. Child trees are recursively constructed by separate calls and then referenced from parent boxes. Allocator addresses may be reused after a short-lived tree is freed; they cannot be shared by simultaneously live node buffers. Empty leaf child sentinels may be shared, while each leaf node buffer is fresh.',
    'proof': [
        proof(G / 'sources/uiua/binarytrees/compact.ua', 1, 15,
              'Build makes two independent recursive calls, each at depth-1; Leaf also calls Tree. Long is constructed before all short-lived batches and traversed afterward. Iteration calls Check TreeAt anew for every element of the requested iteration vector; no tree or check formula is substituted.'),
        proof(U / 'src/compile/data.rs', 383, 419,
              'The three-field boxed Tree data constructor is a callable Node::Array, with one field argument for each of Id, Left and Right.'),
        proof(U / 'src/run.rs', 962, 986,
              'Every runtime constructor invocation drains its field values and creates fresh scalar boxes, then assembles the fields into an array.'),
        proof(U / 'src/array.rs', 517, 521,
              'Each scalar box is a fresh one-element cowslice, not an interned or memoized node.'),
        proof(U / 'src/algorithm/dyadic/combine.rs', 1287, 1291,
              'A boxed array starts with the first freshly created scalar box.'),
        proof(U / 'src/algorithm/dyadic/combine.rs', 1352, 1365,
              'The freshly created first box buffer reserves capacity for all three fields before fields are appended.'),
        proof(U / 'src/algorithm/dyadic/combine.rs', 1405, 1415,
              'The other boxed fields are appended to that new buffer; the node is a three-element Array<Boxed>.'),
        proof(U / 'src/cowslice.rs', 13, 31,
              'cowslice! constructs an ecow vector; node data are reference-counted buffers.'),
        proof(E / 'src/vec.rs', 120, 134,
              'A nonempty ecow buffer allocates storage through grow, beginning with a fresh empty vector.'),
        proof(E / 'src/vec.rs', 531, 556,
              'grow on a new vector invokes alloc::alloc::alloc with a nonzero layout, proving separate simultaneously live backing allocations.'),
        proof(U / 'src/run.rs', 576, 592,
              'The evaluated Long tree is stored in the global binding table as an owned constant; later short-lived iterations cannot release this reference.'),
        proof(U / 'src/run.rs', 511, 536,
              'Reading Long clones its stored value without replacing or dropping the original global binding.'),
        proof(E / 'src/vec.rs', 782, 814,
              'Cloning buffers retains references; buffers are released only when the final reference is dropped. Long and its child buffers remain live through all batches.'),
        proof(U / 'src/compile/pre_eval.rs', 31, 82,
              'Compiler pre-evaluation requires bounded pure expressions. Tree depth comes from runtime args; recursive Build remains a function invoked for each requested tree.'),
        proof(U / 'src/tree.rs', 856, 905,
              'A second visit to a recursive function fails the purity walk, so normal compiler pre-evaluation cannot turn Build into a cached constructed subtree.')
    ],
    'limitation': 'No runtime address trace was collected. The distinct-node and retention findings follow the exact pinned constructor, allocation and ownership implementations.'
}
audit_path = G / 'results/uiua/source-audit.json'
adaptations = {
    'nbody': 'Canonical constants, momentum offset, binary64 pair forces, dt=0.01, all requested updates and two fixed nine-decimal energies; optimized computes NewVelocity once per step and retains that value for position/state assembly.',
    'mandelbrot': 'Each pixel evaluates the real/imaginary escape recurrence for at most 50 iterations, with P4 bytes and row padding. Compact runs all 50 array steps; optimized ends a row when no live lane remains.',
    'spectralnorm': 'Canonical A coefficients recomputed inside every Forward and Transpose call: all 40 matrix/vector products in 20 alternating A-transpose-A applications evaluate all N squared terms. Compact constructs a full coefficient array per product; optimized constructs at most 128 output rows per block. No coefficient matrix is cached between products.',
    'binarytrees': 'Full recursive three-field boxed nodes, including allocated leaves; two separate child constructions per internal node, real checks per tree, stretch tree and retained long-lived tree. See pinned allocation/retention proof.',
    'fasta': 'Canonical ALU and 2n/3n/5n lengths, unscaled binary64 cumulative probabilities and direct seed/139968 division for each random draw. Compact performs sequential RNG updates; optimized computes all chunk transitions with an ordinary affine prefix scan. No RNG-period cache.',
    'knucleotide': 'Parse stdin THREE record and build complete growing maps for every window at all seven widths 1,2,3,4,6,12,18 with generic map insertion. Compact materializes each complete window array; optimized visits at most 65536 start positions per chunk plus k-1 overlap, retaining one complete map across chunks and visiting every overlapping window exactly once. Comments, CRLF, case, short/missing records, lexical ties and a chunk boundary are covered by adversarial evidence.',
    'revcomp': 'Read complete FASTA records, reverse/complement all IUB symbols, uppercase and wrap at 60 columns. Optimized coalesces output writes; record transformation remains Uiua.',
    'regexredux': 'Generic runtime regex operations count all nine case-sensitive patterns and perform all five ordered replacements; initial, cleaned and final lengths included.',
    'fannkuchredux': 'Enumerate every factorial rank as a permutation, perform prefix reversals, signed checksum and maximum. Optimized processes ordinary arrays of 1024 rank lanes; no permutation/flip-result cache.',
    'pidigits': 'Full unbounded-integer spigot state/control in Uiua with two separate (3Q+R)/T and (4Q+R)/T extractions every candidate attempt; generic GMP arithmetic alone crosses FFI. Exact line count and final padding.'
}
environment = {'LC_ALL': 'C', 'OMP_NUM_THREADS': '1', 'OPENBLAS_NUM_THREADS': '1',
               'VECLIB_MAXIMUM_THREADS': '1'}
rows = []
for benchmark in adaptations:
    for name, variant in [('compact', 'compact'), ('fast', 'optimized')]:
        source = G / 'sources/uiua' / benchmark / (name + '.ua')
        source_hash = sha(source)
        validation = small[benchmark + '-uiua-' + name]
        assert validation['source_sha256'] == source_hash
        count, physical, expansions = loc(source.read_text())
        command = [str(R), 'run', '--no-format', str(source)]
        stdin = benchmark in ['knucleotide', 'revcomp', 'regexredux']
        if not stdin:
            command.append('{n}')
        sid = benchmark + '-uiua-' + variant
        library = [gmp] if benchmark == 'pidigits' else []
        execution = {'source_sha256': source_hash, 'runtime_sha256': runtime['sha256'],
                     'command': [token if token != str(source) else '<source>' for token in command],
                     'cwd': str(G), 'runtime_environment': environment,
                     'library_pins': gmp['pins'] if library else {}, 'input_mode': 'stdin' if stdin else 'argument'}
        fingerprint = hashlib.sha256(json.dumps(execution, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        row = {
            'id': sid, 'key': benchmark + ':' + sid, 'benchmark': benchmark,
            'language': 'uiua', 'variant': variant, 'profile': variant,
            'source_profile': name, 'source': str(source), 'source_sha256': source_hash,
            'command': command, 'cwd': str(G), 'input_mode': execution['input_mode'],
            'build_command': [], 'build_status': 'not-required',
            'execution_mode': 'official-compiled-interpreter-runtime',
            'runtime': runtime, 'runtime_sha256': runtime['sha256'],
            'runtime_environment': environment, 'library_support': library,
            'pins': {**runtime['pins'], **(gmp['pins'] if library else {})},
            'source_url': None, 'provenance': 'Locally authored Uiua application source, pinned and visibly counted.',
            'adaptations': [adaptations[benchmark]],
            'algorithm': {'strict': True, 'native_benchmark_kernel': False,
                          'reason': adaptations[benchmark],
                          'arithmetic': 'unbounded-integer' if benchmark == 'pidigits' else 'binary64' if benchmark in ['nbody', 'spectralnorm', 'mandelbrot', 'fasta'] else 'integer'},
            'small_validation': validation,
            'adversarial_validation': [entry for entry in adversarial if entry['benchmark'] == benchmark and entry['variant'] == name],
            'loc': count, 'physical_loc': physical, 'loc_expansions': expansions,
            'loc_review': 'Nonblank noncomment physical source lines; each independent binding expands to its own LOC. Tacit pipelines, branches, long literals and one data declaration remain one expression. No source packing discount.',
            'execution_fingerprint': fingerprint, 'execution_fingerprint_inputs': execution,
            'eligible_for_chart': False, 'measurement_status': 'not-measured',
            'recommended_timeout_seconds': 86400 if benchmark in ['nbody', 'fannkuchredux', 'binarytrees'] else 14400,
            'maximum_workers': 1
        }
        if benchmark == 'binarytrees':
            row['allocation_fixture'] = {'path': str(audit_path), 'proof_type': 'pinned-runtime-source',
                                         'fresh_nodes_including_leaves': True, 'retained_long_lived_tree': True}
        if benchmark == 'pidigits':
            row['support_loc_policy'] = 'Generic new/free/set/copy/add/sub/mul/mul-small/floor-div/compare/get GMP adapter excluded; all spigot/control/output remains counted Uiua.'
        if variant == 'optimized':
            compact = rows[-1]
            if source_hash == compact['source_sha256'] and fingerprint == compact['execution_fingerprint']:
                row['timing_alias_of'] = compact['id']
                row['timing_alias_reason'] = 'Identical source bytes and complete normalized execution fingerprint; only source filename differs.'
        rows.append(row)

manifest = {
    'status': 'prepared', 'language': 'uiua', 'runtime': runtime,
    'pins': {**runtime['pins'], **gmp['pins']},
    'library_support': [gmp],
    'install_pins': str(G / 'uiua-install-pins.json'),
    'install_command': [PY, str(G / 'install_uiua_runtime.py')],
    'preparation_command': [PY, str(G / 'prepare_uiua_manifest.py')],
    'validation_commands': [[PY, str(G / 'validate_uiua.py')],
                            [PY, str(G / 'validate_uiua_adversarial.py')]],
    'validation_evidence': {'small': {'path': str(small_path), 'sha256': sha(small_path), 'passed': 20},
                            'adversarial': {'path': str(adversarial_path), 'sha256': sha(adversarial_path), 'passed': len(adversarial)},
                            'source_audit': str(audit_path)},
    'programs': rows,
    'protocol': 'This metadata/audit preparation executes no benchmark or compiler. Root owns guarded, sequential fresh-process full measurements at unchanged workloads with startup, standard operations, generic GMP/helper loading and all output included.',
    'timeout_advice': 'Conservative ceilings, not timing estimates. Full strict interpreter workloads can take substantial time. Do not extrapolate small correctness runs or report them as full timings.',
    'alias_policy': 'Alias only byte-identical source with equal runtime, command shape, cwd, environment, library pins and input mode.'
}
(G / 'uiua-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
audit = {'status': 'passed-small-adversarial-and-source-audit',
         'all_20_small_cases_pass': True, 'all_adversarial_cases_pass': True,
         'adversarial_case_count': len(adversarial),
         'runtime': runtime, 'library': gmp,
         'source_pins': {row['source']: row['source_sha256'] for row in rows},
         'source_loc': {row['id']: row['loc'] for row in rows},
         'binarytrees': tree_proof, 'pidigits': gmp['division_audit'],
         'full_performance_runs': 0,
         'metadata_support_pins': pins([G / 'prepare_uiua_manifest.py', G / 'install_uiua_runtime.py', G / 'uiua-install-pins.json', G / 'support/uiua/release-0.19.1.json', G / 'validate_uiua.py', G / 'validate_uiua_adversarial.py'])}
audit_path.write_text(json.dumps(audit, indent=2) + '\n')
print(f'Prepared 20 Uiua candidates; 20 small and {len(adversarial)} adversarial passes; allocation/retention source proof saved; metadata script executes no probes or full runs.')
