#!/usr/bin/env python3
"""Plot the measured fastest/smallest endpoints of complete reverse-complement sources."""
import argparse
import json
import math
from pathlib import Path

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

COLORS = {'chapel': '#188754', 'julia': '#9558b2', 'rust': '#aa5724',
          'java': '#326994', 'go': '#008b9b', 'mech': '#e24b4b',
          'c': '#606770', 'cpp': '#c39111'}
LABELS = {'chapel': 'Chapel', 'julia': 'Julia', 'rust': 'Rust',
          'java': 'Java (OpenJDK JIT)', 'go': 'Go',
          'mech': 'Mech (Cranelift AOT)', 'c': 'C (GCC + LLVM)',
          'cpp': 'C++ (GCC + LLVM)'}


def render(path, data, metric, suffix):
    rows = data['programs']
    fastest_time = min(r['median_seconds'] for r in rows)
    smallest_size = min(r['source'][metric] for r in rows)
    fig, ax = plt.subplots(figsize=(11.5, 7.5), layout='constrained')
    ax.set_xscale('log', base=2)
    ax.set_yscale('log', base=2)
    formatter = FuncFormatter(lambda x, _: f'{x:g}×')
    ax.xaxis.set_major_formatter(formatter)
    ax.yaxis.set_major_formatter(formatter)
    ax.grid(which='both', color='#dfe3e8', linewidth=.7)
    ax.set_axisbelow(True)
    table = ['| Language | Fastest source / build | Seconds | LOC | gzip bytes | Smallest source / build | Seconds | LOC | gzip bytes | Time tradeoff |',
             '| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |']
    endpoints = {}
    for language in sorted({r['language'] for r in rows}):
        cohort = [r for r in rows if r['language'] == language]
        fast = min(cohort, key=lambda r: (r['median_seconds'], r['source'][metric]))
        small = min(cohort, key=lambda r: (r['source'][metric], r['median_seconds']))
        color = COLORS[language]
        x = lambda r: r['source'][metric] / smallest_size
        y = lambda r: r['median_seconds'] / fastest_time
        ax.scatter([x(r) for r in cohort], [y(r) for r in cohort],
                   color=color, s=20, alpha=.24)
        ax.plot([x(fast), x(small)], [y(fast), y(small)],
                color=color, linewidth=1.6)
        count = len({r['path'] for r in cohort})
        source_label = 'source' if count == 1 else 'sources'
        ax.scatter(x(fast), y(fast), color=color, s=70,
                   label=f'{LABELS[language]} ({count} {source_label}, {len(cohort)} builds)')
        if fast['id'] != small['id']:
            ax.scatter(x(small), y(small), color=color, marker='s', s=55)
        tradeoff = small['median_seconds'] / fast['median_seconds']
        size_cost = fast['source'][metric] / small['source'][metric]
        endpoints[language] = {
            'fastest': fast['id'], 'smallest': small['id'],
            'time_cost': tradeoff, 'size_cost': size_cost,
            'log2_line_length': math.hypot(math.log2(tradeoff), math.log2(size_cost)),
        }
        table.append(f"| {LABELS[language]} | {fast['id']} | {fast['median_seconds']:.3f} | {fast['source']['nonblank_noncomment_lines']} | {fast['source']['gzip1_bytes']} | {small['id']} | {small['median_seconds']:.3f} | {small['source']['nonblank_noncomment_lines']} | {small['source']['gzip1_bytes']} | {tradeoff:.2f}× |")
        if language == 'mech':
            dot = fast['path'] == small['path']
            note = f"Mech: {fast['source']['nonblank_noncomment_lines']} LOC · {fast['median_seconds']:.3f}s"
            if dot:
                note += '\nFastest and smallest are the same source'
            ax.annotate(note, (x(fast), y(fast)), xytext=(15, 17),
                        textcoords='offset points', fontsize=9, color=color)
    p = data['protocol']
    ax.set_title(f"Reverse-complement: execution time and complete source size\n{data['hardware']['cpu']}; FASTA N={p['fasta_n']}; median of {p['samples']} fresh processes",
                 loc='left', fontsize=15, pad=15)
    unit = 'compressed source size' if metric == 'gzip1_bytes' else 'nonblank, noncomment lines'
    ax.set_xlabel(f'Normalized {unit} — smaller ←', fontsize=11)
    ax.set_ylabel('Normalized elapsed time — faster ↓', fontsize=11)
    ax.legend(loc='upper right', fontsize=8.4, framealpha=.95)
    caption = ('Circle: fastest; square: smallest; faint points: every measured build. Lines join each language’s endpoints.\n'
               'All 250 million bases transformed; regular stdin/stdout files; startup/JIT included; AOT builds excluded.\n'
               'Mech includes its complement table and transform function; general byte/record/IO runtime excluded. Experimental v0.4 extension.\n'
               'Unmodified x86-only sources excluded on ARM; both GCC and LLVM C/C++ builds attempted. All shown outputs pass.\n'
               'Best observed medians in this sweep; desktop timing variability is substantial. Near ties do not establish a winner.')
    if 'measurement_batches' in data:
        caption += '\nTwo separate batches: 5 samples for the competitive cohort; 7 for current Mech and fastest C/Rust controls. See report for provenance.'
    fig.text(.015, -.025, caption, fontsize=9, va='top', color='#424b57')
    stem = path.with_suffix('')
    target = stem.parent / (stem.name + '.corner' + suffix)
    for ext in ('png', 'svg'):
        fig.savefig(str(target) + '.' + ext, dpi=170, bbox_inches='tight')
    plt.close(fig)
    Path(str(target) + '.table.md').write_text('\n'.join(table) + '\n')
    Path(str(target) + '.endpoints.json').write_text(json.dumps({
        'source_report': path.name, 'metric': metric,
        'time_baseline_seconds': fastest_time, 'size_baseline': smallest_size,
        'languages': endpoints,
    }, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('report', type=Path)
    args = parser.parse_args()
    data = json.loads(args.report.read_text())
    render(args.report, data, 'gzip1_bytes', '')
    render(args.report, data, 'nonblank_noncomment_lines', '.loc')


if __name__ == '__main__':
    main()
