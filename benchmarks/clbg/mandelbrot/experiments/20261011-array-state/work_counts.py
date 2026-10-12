"""Bounded recurrence counts only; no elapsed-time estimates or full extrapolation."""
import fcntl
import hashlib
import json
from pathlib import Path
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
LOCK = Path('/tmp/mech-clbg-timing.lock')


def count(n, lanes, interval):
    scale = 2.0 / n
    totals = dict(real_pixel_recurrences=0, padded_pixel_recurrences=0,
                  groups=0, predicate_observations=0, committed_group_updates=0)
    rows = []
    for y in range(n):
        ci = y * scale - 1.0
        groups = []
        for start in range(0, n, lanes):
            real = min(lanes, n - start)
            cr = [x * scale - 1.5 for x in range(start, start + lanes)]
            zr, zi, iteration, observations = [0.0] * lanes, [0.0] * lanes, 0, 0
            while True:
                tr, ti = [z * z for z in zr], [z * z for z in zi]
                inside = [r + i <= 4.0 for r, i in zip(tr, ti)]
                observations += 1
                if iteration >= 50 or not any(inside):
                    break
                for step in range(interval):
                    if step:
                        tr, ti = [z * z for z in zr], [z * z for z in zi]
                    next_zr = [r - i + c for r, i, c in zip(tr, ti, cr)]
                    next_zi = [(r + r) * i + ci for r, i in zip(zr, zi)]
                    zr, zi = next_zr, next_zi
                iteration += interval
            group = dict(column=start, real_lanes=real, padded_lanes=lanes - real,
                         recurrences_per_lane=iteration, predicate_observations=observations,
                         committed_group_updates=iteration // interval,
                         real_pixel_recurrences=real * iteration,
                         padded_pixel_recurrences=(lanes - real) * iteration)
            groups.append(group)
            for name in ('real_pixel_recurrences', 'padded_pixel_recurrences',
                         'predicate_observations', 'committed_group_updates'):
                totals[name] += group[name]
            totals['groups'] += 1
        rows.append(dict(row=y, groups=groups))
    totals['physical_pixel_recurrences'] = totals['real_pixel_recurrences'] + totals['padded_pixel_recurrences']
    return dict(width=n, height=n, lanes=lanes, authored_observation_interval=interval,
                predicate_order='Observe retained z/iteration before committing next state; initial observation at iteration 0 included.',
                totals=totals, rows=rows)


def main():
    with LOCK.open('a+') as lock:
        print('Waiting for shared lock for bounded N129 recurrence proof.', flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        n = 129
        profiles = {name: count(n, lanes, interval) for name, lanes, interval in
                    [('scalar_per_step', 1, 1), ('eight_lanes_per_step', 8, 1),
                     ('eight_lanes_five_step', 8, 5), ('whole_row_per_step', n, 1)]}
        report = dict(created_at=datetime.now(timezone.utc).isoformat(), size=n,
                      scope='Bounded authored recurrence proof only; counts do not predict timing, instruction throughput or full-workload results.',
                      reference_script_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                      profiles=profiles)
        target = HERE / 'work-counts-129.json'
        target.write_text(json.dumps(report, indent=2) + '\n')
        for name, profile in profiles.items():
            print(name, json.dumps(profile['totals'], sort_keys=True), flush=True)


if __name__ == '__main__':
    main()
