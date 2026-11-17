"""Screen sharing only the general matrix-index calculation."""
from pathlib import Path
import screen


def main():
    here = Path(__file__).resolve().parent
    shapes = screen.read(here / 'screen/summary.json')['general-byte-read']['selected_instances']
    graph = screen.read_graph(here / 'screen/observe-before-sharing/blocks.tsv')
    chosen = []
    for shape in shapes:
        node, = [block for block in shape['block_ids']
                 if any(inst.startswith('udiv:') for inst in graph[block].instructions)]
        successor, = graph[node].successors
        assert successor in shape['block_ids']
        chosen.append((node, successor, 0))
    _, _, flags = screen.bound_inputs()
    plain = {key: value for key, value in flags.items() if key != 'MECH_COMPACT_ROOT_TAIL_PROFILE'}
    family = ','.join(':'.join(map(str, match)) for match in chosen)
    plain[screen.ENDPOINT] += ';' + family
    screen.run('geometry-fallback', plain)


if __name__ == '__main__':
    main()
