"""Prepare a separate R loop-invariant affine-coefficient candidate."""
from pathlib import Path
import copy
import json
import hashlib
import sys

HERE = Path(__file__).resolve().parent
G = HERE.parent
sys.path.insert(0, str(G))
import r_loc

HELPER = '''# Generic modular affine-prefix coefficients, independent of the incoming seed.
coefficients <- function(size,multiplier,increment,modulus) {
  multipliers <- rep(multiplier,size)
  increments <- rep(increment,size)
  offset <- 1L
  while (offset < size) {
    suffix <- seq.int(offset+1L,size)
    prefix <- suffix-offset
    increments[suffix] <- (multipliers[suffix]*increments[prefix]+increments[suffix]) %% modulus
    multipliers[suffix] <- (multipliers[suffix]*multipliers[prefix]) %% modulus
    offset <- offset*2L
  }
  list(multipliers=multipliers,increments=increments)
}
'''


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    manifest_path = G / 'r-manifest.json'
    manifest = json.loads(manifest_path.read_text())
    original = next(row for row in manifest['programs']
                    if row['benchmark'] == 'fasta' and row['variant'] == 'optimized')
    source_path = Path(original['source'])
    assert sha(source_path) == original['source_sha256']
    text = source_path.read_text()
    start = text.index('      multipliers <- rep(3877.0,size)\n')
    end = text.index('      random <- seeds/139968\n', start)
    old_body = text[start:end]
    assert 'offset <- offset*2L' in old_body
    text = text[:start] + '      index <- seq_len(size)\n      seeds <- (coefs$multipliers[index]*seed+coefs$increments[index]) %% 139968\n' + text[end:]
    text = text.replace('  block <- 60L*4096L\n', '  block <- 60L*4096L\n  coefs <- coefficients(block,3877.0,29573.0,139968.0)\n')
    text = HELPER + text
    assert text.count('coefs <- coefficients(') == 1
    path = HERE / 'sources/fasta.optimized.R'
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    info = r_loc.normalize(text)
    row = copy.deepcopy(original)
    row.update(id='fasta-r-affine-hoisted-optimized-v2',
        key='fasta:fasta-r-affine-hoisted-optimized-v2', source=str(path), source_sha256=sha(path),
        command=[argument.replace(str(source_path), str(path)) for argument in original['command']],
        loc=info['normalized_loc'], physical_loc=info['physical_loc'], loc_expansions=info['expansions'],
        measurement_status='prepared-pending-locked-validation', eligible_for_chart=False,
        samples_seconds=[], sample_validation=[], attempts=[],
        original_source=str(source_path), original_source_sha256=original['source_sha256'],
        supersedes_keys=[original['key']])
    row.pop('small_validation', None)
    row.pop('execution_fingerprint', None)
    row['adaptations'] = [
        'Ordinary generic modular-affine prefix coefficient arrays are calculated once for the fixed245760-draw maximum chunk capacity; this seed-independent loop-invariant work is hoisted across both generated records.',
        'Every chunk materializes every ordered live seed from its incoming seed and the sliced coefficient prefix; no period/state/output cache or known-answer shortcut.',
        'Retain every binary64 seed/139968 division, unscaled sequential CDF, strict findInterval threshold, ALU/header/60-column output and sequential IUB-to-Human seed state.',
        'All generic coefficient helper/application statements count; ordinary R arrays/arithmetic only, no native benchmark executor.']
    row['algorithm'] = dict(strict=True, native_benchmark_kernel=False, arithmetic='binary64',
        reason='The same complete modular recurrence is reconstructed for every requested draw using generic affine coefficients. Only seed-independent coefficient preparation is loop-invariant; every live seed, binary64 division, CDF lookup and output byte is evaluated at runtime.',
        chunk_capacity=245760, period_cache=False, output_cache=False,
        coefficient_cache='One seed-independent generic prefix table, sliced at tails; not a table of seeds or output.')
    (HERE / 'manifest.json').write_text(json.dumps(dict(status='prepared-pending-locked-validation',
        language='r', programs=[row], runtime=manifest['runtime'],
        preserved_pins={str(manifest_path): sha(manifest_path), str(source_path): sha(source_path)},
        source_recipe=dict(path=str(Path(__file__).resolve()), sha256=sha(Path(__file__)))), indent=2) + '\n')
    print(row['key'], row['loc'], 'LOC', row['source_sha256'])


if __name__ == '__main__':
    main()
