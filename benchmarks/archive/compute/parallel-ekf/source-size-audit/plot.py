#!/usr/bin/env python3
"""Draw the four-count EKF source comparison from counts.json."""
from pathlib import Path
import json
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter
from matplotlib.patches import Patch

ROOT = Path(__file__).resolve().parent

def draw(show=False):
    counts = json.loads((ROOT/'counts.json').read_text(encoding='utf-8'))
    mech = counts['ekf.mec']['normalized_characters']
    rust_textbook = counts['rust_textbook.rs']['normalized_characters']
    rust_simd = counts['rust_simd.rs']['normalized_characters']
    plt.rcParams.update({'font.family': 'DejaVu Sans', 'svg.fonttype': 'none', 'pdf.fonttype': 42})
    yellow, tan = '#F4C430', '#DEA584'
    ink, muted, rule = '#202124', '#555B61', '#E5E7E9'
    fig = plt.figure(figsize=(12.8, 7.6), facecolor='white')
    ax = fig.add_axes([0.24, 0.27, 0.69, 0.46], facecolor='white')
    positions = [1.18, 0]
    for values, offset, color in [([mech, mech], .18, yellow), ([rust_textbook, rust_simd], -.18, tan)]:
        bars = ax.barh([p+offset for p in positions], values, height=.29,
                       color=color, edgecolor='none', zorder=3)
        for bar, value in zip(bars, values):
            ax.text(value+75, bar.get_y()+bar.get_height()/2, f'{value:,}',
                    ha='left', va='center', fontsize=16, fontweight='semibold', color=ink)
    ax.set_xlim(0, 6000)
    ax.set_ylim(-.57, 1.75)
    ax.set_yticks(positions, ['Textbook', 'SIMD-4\n8 workers'])
    ax.tick_params(axis='y', length=0, pad=22, labelsize=16, colors=ink)
    ax.set_xticks(range(0, 6001, 1000))
    ax.xaxis.set_major_formatter(FuncFormatter(lambda value, _: f'{value:,.0f}'))
    ax.tick_params(axis='x', length=0, pad=11, labelsize=12, colors=muted)
    ax.grid(axis='x', color=rule, linewidth=.8, zorder=0)
    ax.set_axisbelow(True)
    for spine in ax.spines.values():
        spine.set_visible(False)
    ax.spines['bottom'].set_visible(True)
    ax.spines['bottom'].set_color(rule)
    ax.set_xlabel('Normalized source characters', fontsize=14, labelpad=17, color=ink)
    fig.text(.055, .917, 'EKF application source size', fontsize=26,
             fontweight='bold', color=ink, ha='left')
    fig.text(.055, .864, 'Bearing-only f32 EKF · Application-authored source',
             fontsize=13, color=muted, ha='left')
    fig.legend(handles=[Patch(facecolor=yellow, label='Mech'), Patch(facecolor=tan, label='Rust')],
               loc='upper left', bbox_to_anchor=(.238, .817), frameon=False,
               ncol=2, fontsize=13, handlelength=1.7, columnspacing=2.3)
    fig.text(.055, .12,
             'Includes initialization, numerical code, validation, and application-written execution support.',
             fontsize=10.5, color=muted)
    fig.text(.055, .085,
             'Excludes benchmark code, comments, and whitespace outside literals. Each chosen name counts as one character.',
             fontsize=10.5, color=muted)
    fig.text(.055, .041,
             'Same Mech source in both cases. Rust textbook includes the block-rollback correction; SIMD source is unchanged.',
             fontsize=10.5, color=ink)
    for ext in ('png','svg','pdf'):
        fig.savefig(ROOT/f'ekf-source-size-reconciled.{ext}', dpi=300, facecolor='white')
    if show:
        plt.show()
    else:
        plt.close(fig)
    return fig

if __name__ == '__main__':
    draw()
