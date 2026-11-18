"""Stage a same-plan reuse probe for the Complete and Fueled inventories."""

from difflib import unified_diff
from pathlib import Path
import subprocess

from build import read, save, sha
from narrow import SOURCE
from stage import BASE, ROOT

RELATIVE = Path('hosts/gpu/src/native_artifact/program/prepare/phase_numeric.rs')
FLAG = 'MECH_REUSE_NUMERIC_PHASE_INVENTORY'
TRACE = 'MECH_TRACE_NUMERIC_PHASE_INVENTORY'


def replace(source, before, after):
    assert source.count(before) == 1, before
    return source.replace(before, after)


def main():
    input_text = subprocess.check_output(['git', 'show', f'{BASE}:{RELATIVE}'], cwd=ROOT).decode()
    begin = '''    let plan = facts.plan();
    derive_with_children(
        facts,
        tail,
        frames,
        aggregates,
        calls,
        visits,
        Coverage::Complete,
    )
    .or_else(|reason| {
        let cold = || {
            derive_with_children(
                facts,
                tail,
                frames,
                aggregates,
                calls,
                visits,
                Coverage::ScalarCold,
            )
        };
        let fueled = || derive_fueled(facts, tail, frames, aggregates, calls, visits);'''
    change = '''    let plan = facts.plan();
    let reuse = std::env::var_os("MECH_REUSE_NUMERIC_PHASE_INVENTORY").is_some();
    let trace = std::env::var_os("MECH_TRACE_NUMERIC_PHASE_INVENTORY").is_some();
    let mut cached = None;
    let mut attempt = |mode| {
        if !reuse && !trace {
            return match mode {
                Coverage::Fueled => derive_fueled(facts, tail, frames, aggregates, calls, visits),
                Coverage::Hybrid => derive_hybrid(facts, tail, frames, aggregates, calls, visits),
                Coverage::Intervals => derive_interval(facts, tail, frames, aggregates, calls, visits),
                _ => derive_with_children(facts, tail, frames, aggregates, calls, visits, mode),
            };
        }
        derive_with_cached_inventory(
            facts, tail, frames, aggregates, calls, visits, mode, reuse.then_some(&mut cached), trace,
        )
    };
    attempt(Coverage::Complete).or_else(|reason| {'''
    updated = replace(input_text, begin, change)
    updated = replace(updated, '''        let selected = if scalar_state_child {
            fueled().or_else(|_| cold())
        } else {
            cold().or_else(|_| fueled())
        };
        selected
            .or_else(|_| derive_hybrid(facts, tail, frames, aggregates, calls, visits))
            .or_else(|_| derive_interval(facts, tail, frames, aggregates, calls, visits))
            .map_err(|_| reason)''',
                      '''        let (first, second) = if scalar_state_child {
            (Coverage::Fueled, Coverage::ScalarCold)
        } else {
            (Coverage::ScalarCold, Coverage::Fueled)
        };
        attempt(first)
            .or_else(|_| attempt(second))
            .or_else(|_| attempt(Coverage::Hybrid))
            .or_else(|_| attempt(Coverage::Intervals))
            .map_err(|_| reason)''')
    driver = '''    mode: Coverage,
) -> Result<Proof, &'static str> {
    let plan = facts.plan();
    let fields = frames'''
    updated = replace(updated, driver, '''    mode: Coverage,
) -> Result<Proof, &'static str> {
    derive_with_cached_inventory(facts, tail, frames, aggregates, calls, visits, mode, None, false)
}

// Complete and Fueled visit exactly the same Tail graph. Coverage changes
// how child proofs are funded after discovery. A prior layout rejection is
// shared only within this immutable root and never becomes an admission.
type Inventory = Result<super::numeric_phase::Forest, &'static str>;

fn derive_with_cached_inventory(
    facts: &NativeProgramFacts<'_>,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
    mode: Coverage,
    cached: Option<&mut Option<Inventory>>,
    trace: bool,
) -> Result<Proof, &'static str> {
    let plan = facts.plan();
    let fields = frames''')
    updated = replace(updated, '''    let forest = match mode {
        Coverage::Complete => super::numeric_phase::Forest::derive(plan, tail)?,
        Coverage::ScalarCold => super::numeric_phase::Forest::with_cold_children(plan, tail)?,
        Coverage::Intervals => super::numeric_phase::Forest::intervals(plan, tail)?,
        Coverage::Fueled => super::numeric_phase::Forest::fueled(plan, tail)?,
        Coverage::Hybrid => super::numeric_phase::Forest::hybrid(plan, tail)?,
    };''',
                      '''    use super::numeric_phase::Forest;
    let started = trace.then(std::time::Instant::now);
    let mut reused = false;
    let ordinary;
    let forest = match (mode, cached) {
        (Coverage::Complete | Coverage::Fueled, Some(cache)) => {
            reused = cache.is_some();
            match cache.get_or_insert_with(|| Forest::derive(plan, tail)) {
                Ok(forest) => {
                    forest.checked_children = mode == Coverage::Fueled;
                    Ok(&*forest)
                }
                Err(reason) => Err(*reason),
            }
        }
        _ => {
            ordinary = match mode {
                Coverage::Complete => Forest::derive(plan, tail),
                Coverage::ScalarCold => Forest::with_cold_children(plan, tail),
                Coverage::Intervals => Forest::intervals(plan, tail),
                Coverage::Fueled => Forest::fueled(plan, tail),
                Coverage::Hybrid => Forest::hybrid(plan, tail),
            };
            ordinary.as_ref().map_err(|reason| *reason)
        }
    };
    if let Some(started) = started {
        eprintln!("MECH_PHASE_INVENTORY\\t{}\\t{}\\t{}\\t{}\\t{}",
            tail.header(), mode as u8, reused, started.elapsed().as_nanos(),
            forest.as_ref().map_or(0, |forest| forest.sites.len()));
    }
    let forest = forest?;''')
    updated = replace(updated, 'prepaid_sites: forest.prepaid_sites.into_iter().collect(),',
                      'prepaid_sites: forest.prepaid_sites.iter().copied().collect(),')
    target = SOURCE / 'source' / RELATIVE
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(updated)
    pins = read(SOURCE / 'source.json')
    assert pins['base'] == BASE
    pins['source_sha256'][str(RELATIVE)] = sha(target)
    pins['inventory_flag'] = FLAG
    pins['inventory_trace'] = TRACE
    patch = ''.join(unified_diff(input_text.splitlines(keepends=True), updated.splitlines(keepends=True),
                                 f'a/{RELATIVE}', f'b/{RELATIVE}'))
    (SOURCE / 'phase-inventory.patch').write_text(patch)
    bundle = SOURCE / 'cursor.patch'
    bundle.write_text(bundle.read_text() + patch)
    save(SOURCE / 'source.json', pins)


if __name__ == '__main__':
    main()
