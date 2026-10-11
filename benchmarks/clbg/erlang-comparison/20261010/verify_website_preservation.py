"""Verify that adding Erlang preserves every previously published measurement/source."""
import json
from pathlib import Path

D = Path(__file__).resolve().parent
SITE = Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site/dist/data.json')


def main():
    before = json.loads((D / 'results/website-baseline.json').read_text())
    after = json.loads(SITE.read_text())
    for key, program in before['programs'].items():
        assert after['programs'][key] == program, key
    for key, source in before['files'].items():
        assert after['files'][key] == source, key
    for language in before['languages']:
        current = next(l for l in after['languages'] if l['key'] == language['key'])
        assert current == language, language['key']
    new = set(after['programs']) - set(before['programs'])
    assert new and all(after['programs'][key]['language'] == 'erlang' for key in new)
    print('Preserved all', len(before['programs']), 'previous programs,', len(before['files']), 'sources and', len(before['languages']), 'language entries; added', len(new), 'Erlang endpoints')


if __name__ == '__main__':
    main()
