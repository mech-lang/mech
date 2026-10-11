#!/usr/bin/env python3
"""Export LOC-only mega and Mech spread plots from the audited AC rerun."""
import json
import math
import statistics
from pathlib import Path

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Patch
from matplotlib.ticker import FuncFormatter, LogLocator
import numpy as np

from build_original_style import LABELS, models
from language_colors import COLORS, LINE_COLORS, OUTLINE_COLORS, MECH_PROFILES

HERE = Path(__file__).resolve().parent
RATIO = FuncFormatter(lambda value, _: f'{value:g}×')


def same_point(first, second):
    return first and all(math.isclose(first[k], second[k]) for k in ('x', 'y'))


def canvas(title, subtitle):
    fig, ax = plt.subplots(figsize=(13, 9), facecolor='white')
    ax.set_facecolor('white')
    fig.subplots_adjust(left=.09, right=.985, top=.88, bottom=.19)
    ax.set_xscale('log', base=2)
    ax.set_yscale('log', base=2)
    for axis in (ax.xaxis, ax.yaxis):
        axis.set_major_locator(LogLocator(base=2, numticks=12))
        axis.set_major_formatter(RATIO)
    ax.grid(color='#dfe3e8', linewidth=.7)
    ax.set_axisbelow(True)
    ax.set_title(title + '\n' + subtitle, loc='left', fontsize=17, pad=16)
    ax.set_xlabel('Source lines / fewest source lines (log₂ scale) — fewer ←', fontsize=12)
    ax.set_ylabel('Elapsed time / fastest elapsed time (log₂ scale) — faster ↓', fontsize=12)
    return fig, ax


def limits(ax, points):
    ax.set_xlim(min(p['x'] for p in points) * .91, max(p['x'] for p in points) * 1.14)
    ax.set_ylim(min(p['y'] for p in points) * .83, max(p['y'] for p in points) * 1.25)


def footer(fig, lines):
    fig.text(.05, .055, '\n'.join(lines), fontsize=9, color='#505863', va='bottom')


def save(fig, stem):
    for extension in ('png', 'svg'):
        fig.savefig(HERE / f'{stem}.{extension}', dpi=160, facecolor='white')


def export_mega(data):
    fig, ax = canvas('Language summary across all eight benchmarks: LOC and speed',
                     'Bold: language geometric means · faint: individual benchmark inputs')
    groups = data['summary']['groups']
    points = [p for g in groups for p in (g['fast'], g['small']) if p]
    inputs = [p for study in data['studies'] for group in study['groups']
              for p in (group['fast'], group['small']) if p]
    limits(ax, [*points, *inputs, dict(x=1, y=1)])
    input_collections = 0
    for group in groups:
        lang = group['lang']
        contributors = [next(g for g in study['groups'] if g['lang'] == lang)
                        for study in data['studies']]
        for kind, marker in (('fast', 'o'), ('small', 's')):
            samples = [g[kind] for g in contributors if g[kind] is not None]
            if not samples:
                continue
            assert len(samples) == 8
            dots = ax.scatter([p['x'] for p in samples], [p['y'] for p in samples],
                              marker=marker, s=29, color=COLORS[lang],
                              alpha=.27 if lang == 'mech' else .20, zorder=2)
            dots.set_gid(f'mean-input-{lang}-{kind}')
            input_collections += 1
    for group in groups:
        lang = group['lang']
        color = COLORS[lang]
        fast, small = group['fast'], group['small']
        size, layer = (95, 6) if lang == 'mech' else (67, 4)
        if fast and not same_point(fast, small):
            line, = ax.plot([small['x'], fast['x']], [small['y'], fast['y']],
                            color=LINE_COLORS[lang], linewidth=2.4 if lang == 'mech' else 1.8,
                            zorder=layer - 1)
            line.set_gid(f'geometric-mean-line-{lang}')
        if fast:
            ax.scatter(fast['x'], fast['y'], marker='o', s=size,
                       color=color, edgecolor=OUTLINE_COLORS[lang], linewidth=.7, zorder=layer)
        if not same_point(fast, small):
            ax.scatter(small['x'], small['y'], marker='s', s=size,
                       color=color, edgecolor=OUTLINE_COLORS[lang], linewidth=.7, zorder=layer)
    expected_lines = sum(g['fast'] is not None and not same_point(g['fast'], g['small']) for g in groups)
    expected_marks = sum((g['fast'] is not None) + (not same_point(g['fast'], g['small'])) for g in groups)
    assert len(ax.lines) == expected_lines
    assert len(ax.collections) == expected_marks + input_collections
    handles = []
    for lang, label in LABELS.items():
        compact = next(g for g in groups if g['lang'] == lang)['compact_only']
        handles.append(Line2D([], [], color=LINE_COLORS[lang], markerfacecolor=COLORS[lang], markeredgecolor=OUTLINE_COLORS[lang], marker='s' if compact else 'o',
                              linestyle='None', markersize=7,
                              label=label + (' · fewest LOC only' if compact else '')))
    ax.legend(handles=handles, loc='upper right', fontsize=8.4, framealpha=.96)
    mech = next(g for g in groups if g['lang'] == 'mech')['fast']
    ax.annotate(f'Mech: {mech["x"]:.2f}× LOC · {mech["y"]:.2f}× time',
                (mech['x'], mech['y']), xytext=(12, 12), textcoords='offset points',
                fontsize=10, color=LINE_COLORS['mech'])
    footer(fig, [
        'Bold circles/squares: fastest/fewest-LOC geometric means; one line per language. Faint matching markers: individual benchmark inputs.',
        'Normalize within each benchmark, then take equal-weight geometric means over all 8 benchmarks. Logarithmic axes.',
        'Apple M3 · AC power · Low Power Mode off · 3 fresh processes/build · startup and full output included.',
        'Added scripting runtimes: fewest-LOC entries only; fastest entries not sampled. LOC excludes blanks and comment-only lines.',
        'Complete Mech source counted; generic runtime excluded. Historical algorithm and allocation differences retained.'
    ])
    save(fig, 'mega-summary.loc')
    plt.close(fig)
    return dict(aggregation='equal-weight geometric means over eight benchmarks',
                languages=len(groups), lines=expected_lines, markers=expected_marks,
                compact_only=sum(g['compact_only'] for g in groups), build_dots=0,
                benchmark_input_points=len(inputs), benchmark_input_collections=input_collections,
                fastest_input_points=sum(g['fast'] is not None for s in data['studies'] for g in s['groups']),
                fewest_loc_input_points=sum(g['small'] is not None for s in data['studies'] for g in s['groups']))


def spread(points):
    """One-SD covariance contour in log-ratio space, centered at geometric means."""
    values = np.array([[p['x'], p['y']] for p in points], dtype=float)
    logged = np.log(values)
    center = logged.mean(axis=0)
    covariance = np.cov(logged, rowvar=False, ddof=1)
    eigenvalues, eigenvectors = np.linalg.eigh(covariance)
    angles = np.linspace(0, 2 * np.pi, 361)
    unit = np.vstack([np.cos(angles), np.sin(angles)])
    log_boundary = center[:, None] + eigenvectors @ np.diag(np.sqrt(np.maximum(eigenvalues, 0))) @ unit
    boundary = np.exp(log_boundary).T
    return dict(n=len(points), center=dict(x=float(np.exp(center[0])), y=float(np.exp(center[1]))),
                log_covariance=covariance.tolist(), log_sample_sd=np.sqrt(np.diag(covariance)).tolist(),
                geometric_sd=np.exp(np.sqrt(np.diag(covariance))).tolist(),
                boundary=[dict(x=float(x), y=float(y)) for x, y in boundary])


def annotate_without_overlap(fig, ax, specs, marker_points, legend):
    """Place benchmark labels inside the plot and clear of labels and markers."""
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    frame = ax.get_window_extent(renderer)
    occupied = [legend.get_window_extent(renderer).expanded(1.02, 1.03)]
    markers = [ax.transData.transform((p['x'], p['y'])) for p in marker_points]
    fallback = [(8, 14), (8, -24), (-8, 14), (-8, -24), (24, 32), (-24, 32),
                (24, -42), (-24, -42), (65, 20), (-65, 20), (65, -48),
                (-65, -48), (12, 70), (-12, 70), (12, -75), (-12, -75)]
    for text, point, preferred in specs:
        for dx, dy in [preferred, *fallback]:
            annotation = ax.annotate(text, (point['x'], point['y']), xytext=(dx, dy),
                                     textcoords='offset points', fontsize=10,
                                     ha='left' if dx >= 0 else 'right', va='bottom' if dy >= 0 else 'top',
                                     zorder=10, color='#222222',
                                     bbox=dict(facecolor='white', edgecolor='none', alpha=.85, pad=1))
            fig.canvas.draw()
            bounds = annotation.get_window_extent(renderer).expanded(1.04, 1.13)
            fits = frame.contains(bounds.x0, bounds.y0) and frame.contains(bounds.x1, bounds.y1)
            clear = all(not bounds.overlaps(b) for b in occupied)
            clear = clear and all(not bounds.padded(5).contains(x, y) for x, y in markers)
            if fits and clear:
                occupied.append(bounds)
                if abs(dx) + abs(dy) > 45:
                    ax.annotate('', (point['x'], point['y']), xytext=(dx, dy), textcoords='offset points',
                                arrowprops=dict(arrowstyle='-', color='#777777', linewidth=.6, alpha=.6), zorder=3)
                break
            annotation.remove()
        else:
            raise RuntimeError(f'Cannot place benchmark label without overlap: {text}')
    return len(occupied) - 1


def export_mech(data):
    groups = [(study, next(g for g in study['groups'] if g['lang'] == 'mech')) for study in data['studies']]
    regions = {kind: spread([g[kind] for _, g in groups]) for kind in ('fast', 'small')}
    fig, ax = canvas('Mech: all eight benchmark tradeoffs and standard deviation',
                     'AOT · geometric means and spread across benchmarks')
    colors = MECH_PROFILES
    for kind in ('small', 'fast'):
        region = regions[kind]
        ax.fill([p['x'] for p in region['boundary']], [p['y'] for p in region['boundary']],
                color=colors[kind], alpha=.16, linewidth=0, zorder=1)
        ax.plot([p['x'] for p in region['boundary']], [p['y'] for p in region['boundary']],
                color=colors[kind], linewidth=1, alpha=.65,
                linestyle='--' if kind == 'small' else '-', zorder=2)
    endpoints = []
    specs = []
    labels = dict(nbody='N-body', mandelbrot='Mandelbrot', knucleotide='K-nucleotide',
                  spectralnorm='Spectral norm', revcomp='Reverse complement',
                  fannkuchredux='Fannkuch-redux', fasta='FASTA', binarytrees='Binary trees')
    for study, group in groups:
        fast, small = group['fast'], group['small']
        endpoints.extend([fast, small])
        same = same_point(fast, small)
        if not same:
            ax.plot([small['x'], fast['x']], [small['y'], fast['y']], color=LINE_COLORS['mech'], linewidth=1.6, zorder=4)
            ax.scatter(small['x'], small['y'], marker='s', s=67, color=COLORS['mech'], edgecolor=LINE_COLORS['mech'], linewidth=.7, zorder=5)
            specs.append((f'{labels[study["key"]]} ({small["loc"]} LOC)', small, (10, 16)))
        ax.scatter(fast['x'], fast['y'], marker='o', s=67, color=COLORS['mech'], edgecolor=LINE_COLORS['mech'], linewidth=.7, zorder=5)
        if study['key'] not in ('knucleotide', 'spectralnorm'):
            specs.append((f'{labels[study["key"]]} ({fast["loc"]} LOC)', fast, (10, -24) if not same else (10, 14)))
    shared = next(g for s, g in groups if s['key'] == 'knucleotide')['fast']
    spectral = next(g for s, g in groups if s['key'] == 'spectralnorm')['fast']
    assert same_point(shared, spectral)
    specs.append(('K-nucleotide (17 LOC)\nSpectral norm (13 LOC)', shared, (-12, -35)))
    handles = []
    for kind, name, marker in [('fast', 'Fastest', 'o'), ('small', 'Fewest LOC', 's')]:
        center = regions[kind]['center']
        ax.scatter(center['x'], center['y'], color=COLORS['mech'], edgecolor='#222222',
                   linewidth=1.1, marker=marker, s=155, zorder=8)
        endpoints.append(center)
        handles.append(Line2D([], [], marker=marker, markerfacecolor=COLORS['mech'],
                              markeredgecolor='#222222', linestyle='None', markersize=10,
                              label=f'{name} geometric mean: {center["x"]:.2f}× LOC · {center["y"]:.2f}× time'))
    handles.extend([Patch(facecolor=colors['fast'], alpha=.16, label='Fastest profile: one standard deviation'),
                    Patch(facecolor=colors['small'], alpha=.16, label='Fewest-LOC profile: one standard deviation')])
    all_points = [*endpoints, *regions['fast']['boundary'], *regions['small']['boundary']]
    limits(ax, all_points)
    ax.set_xlim(min(p['x'] for p in all_points) * .80, max(p['x'] for p in all_points) * 1.25)
    ax.set_ylim(min(p['y'] for p in all_points) * .80, max(p['y'] for p in all_points) * 1.20)
    ax.set_xticks([v for v in (.75, 1, 1.25, 1.5, 2, 2.5) if ax.get_xlim()[0] <= v <= ax.get_xlim()[1]])
    ax.set_yticks([v for v in (.75, 1, 1.25, 1.5, 2, 3, 4, 6) if ax.get_ylim()[0] <= v <= ax.get_ylim()[1]])
    legend = ax.legend(handles=handles, loc='upper right', fontsize=9, framealpha=.96)
    placed = annotate_without_overlap(fig, ax, specs, endpoints, legend)
    footer(fig, [
        'Circle: fastest measured; square: fewest LOC. A single circle means the two endpoints coincide. Large outlined markers: geometric means.',
        'Shaded regions: one sample standard deviation in log-ratio space across the 8 benchmarks; covariance retained (n = 8).',
        'This describes cross-benchmark spread, not run-to-run timing uncertainty or a confidence interval. Axes are logarithmic.',
        'Normalize each benchmark to the measured cohort’s fewest LOC and fastest median. Apple M3 · AC power · Low Power Mode off.',
        '3 fresh processes/build; startup and full output included. LOC excludes blanks and comment-only lines; generic Mech runtime excluded.'
    ])
    save(fig, 'mech-all-benchmarks.loc.stddev')
    plt.close(fig)
    return regions, placed


def main():
    data = models()
    audit = json.loads((HERE.parent / 'retime/results/audit.json').read_text())
    report = json.loads((HERE.parent / 'retime/results' / audit['report']).read_text())
    lookup = {(p['benchmark'], p['id']): p for p in report['programs']}
    for study in data['studies']:
        for group in study['groups']:
            for point in [*group['builds'], group['fast'], group['small']]:
                if not point:
                    continue
                row = lookup[(study['key'], point['id'])]
                assert point['loc'] == row['source']['nonblank_noncomment_lines']
                assert point['seconds'] == statistics.median(row['samples_seconds'])
                assert len(row['samples_seconds']) == 3
    data['mega_chart'] = export_mega(data)
    regions, placed = export_mech(data)
    data['mech_spread'] = regions
    data['spread_method'] = 'One-SD covariance ellipse of natural-log endpoint ratios; sample covariance ddof=1 across 8 benchmarks. Not timing uncertainty.'
    (HERE / 'mega-loc-endpoints.json').write_text(json.dumps(data, indent=2) + '\n')
    print(f'Exported LOC language summary ({data["mega_chart"]["lines"]} geometric-mean lines, {data["mega_chart"]["benchmark_input_points"]} faint benchmark inputs) and Mech-only chart (8 benchmarks, {placed} labels); audited raw medians retained.')


if __name__ == '__main__':
    main()
