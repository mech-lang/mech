"""Additive v4 diagnostic String capability repair; scored work is unchanged."""
from generate_traces_base import TRACE, instrument as base_instrument


def instrument(source, benchmark, variant):
    source, changes = base_instrument(source, benchmark, variant)
    if benchmark == 'knucleotide':
        types = 'String, U64' if variant == 'compact' else 'U64, U64'
        key = 'word' if variant == 'compact' else 'Packed.decode(word, width)'
        old_key = 'word' if variant == 'compact' else 'Packed.decode(word, _width)'
        old = ('      Trace("MAP_DONE\\t" + _width.string() + "\\t" + counts.size().string())\n'
               '      for (word, count) in counts.pairs() do\n'
               '        Trace("MAP\\t" + _width.string() + "\\t" + ' + old_key + ' + "\\t" + count.string())\n'
               '      end\n'
               '      _parent.done(_slot, consume counts)\n')
        new = '      _parent.done(_slot, ObservedMap(consume counts, _width))\n'
        assert source.count(old) == 1, ('Nonunique failed iso observer hook', variant)
        source = source.replace(old, new)
        changes.append(dict(old=old, new=new, kind='consume-same-isolated-map-through-observer'))
        helper = ('''primitive ObservedMap
  fun apply(input: Map[''' + types + '''] iso, width: USize): Map[''' + types + '''] iso^ =>
    recover iso
      let table: Map[''' + types + '''] ref = consume input
      Trace("MAP_DONE\\t" + width.string() + "\\t" + table.size().string())
      for (word, count) in table.pairs() do
        Trace("MAP\\t" + width.string() + "\\t" + ''' + key + ''' + "\\t" + count.string())
      end
      table
    end

''')
        assert source.count('primitive Trace\n') == 1 and 'primitive ObservedMap\n' not in source
        source = source.replace('primitive Trace\n', helper + 'primitive Trace\n')
        changes.append(dict(kind='isolated-observer-recover-scope', inserted=helper,
            ownership='Consume input iso to a local ref only inside recover iso; return the same table as iso^ before the original parent handoff.',
            no_map_copy_or_rebuild=True, production_operations_changed=False))
        return source, changes
    if benchmark != 'binarytrees':
        return source, changes

    def replace(old, new):
        nonlocal source
        assert source.count(old) == 1, ('Nonunique v2 diagnostic hook', old)
        source = source.replace(old, new)
        changes.append(dict(old=old, new=new))

    # Initialize the cache field before digestof this is evaluated after all
    # original left/right fields have been initialized. Finalizer allocates no
    # strings and sends no actor messages, exactly as in the preserved v1 copy.
    replace('  let _trace_final: String\n', '  var _trace_final: String = ""\n')
    replace('    _trace_final = "FINAL\\t" + depth.string() + "\\n"\n', '')
    replace('    end\n  fun _final() =>\n', '''    end
    let identity: String val = (digestof this).string()
    _trace_final = "FINAL\\t" + depth.string() + "\\t" + identity + "\\n"
    match (left, right)
    | (let first: Node box, let second: Node box) =>
      Trace("GRAPH\\t" + depth.string() + "\\t" + identity + "\\t" + (digestof first).string() + "\\t" + (digestof second).string())
    else
      Trace("GRAPH\\t" + depth.string() + "\\t" + identity + "\\t0\\t0")
    end
  fun _final() =>
''')
    replace('    Trace("CHECK\\t" + _trace_depth.string())\n',
            '    Trace("CHECK\\t" + _trace_depth.string() + "\\t" + (digestof this).string())\n')
    replace('    let stretch = Node(_maximum + 1)\n',
            '    let stretch = Node(_maximum + 1)\n    Trace("ROOT_STRETCH_NEW\\t" + (_maximum + 1).string() + "\\t" + (digestof stretch).string())\n')
    replace('    _long = Node(_maximum)\n',
            '    Trace("ROOT_STRETCH_CHECK\\t" + (_maximum + 1).string() + "\\t" + (digestof stretch).string())\n    _long = Node(_maximum)\n    Trace("ROOT_LONG_NEW\\t" + _maximum.string() + "\\t" + (digestof _long).string())\n')
    replace('      let tree = Node(_depth)\n',
            '      let tree = Node(_depth)\n      Trace("ROOT_CHURN_NEW\\t" + _depth.string() + "\\t" + (digestof tree).string() + "\\t" + @pthread_self().string())\n')
    replace('      _sum = _sum + tree.check()\n',
            '      _sum = _sum + tree.check()\n      Trace("ROOT_CHURN_CHECK\\t" + _depth.string() + "\\t" + (digestof tree).string() + "\\t" + @pthread_self().string())\n')
    replace('      _parent.done(_sum)\n',
            '      Trace("WORKER_DONE\\t" + _depth.string() + "\\t" + (digestof this).string())\n      _parent.done(_sum, _depth, digestof this)\n')
    replace('  be done(value: U64) =>\n',
            '  be done(value: U64, trace_depth: USize, trace_worker: USize) =>\n')
    replace('    _sum = _sum + value\n',
            '    Trace("WORKER_JOIN\\t" + trace_depth.string() + "\\t" + trace_worker.string())\n    _sum = _sum + value\n')
    replace('        _env.out.flush()\n',
            '        Trace("ROOT_LONG_CHECK\\t" + _maximum.string() + "\\t" + (digestof _long).string())\n        _env.out.flush()\n')
    return source, changes
