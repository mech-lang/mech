"""Unexecuted v2 candidate: same work hooks plus actual tree object identities."""
from generate_traces_base import TRACE, instrument as base_instrument


def instrument(source, benchmark, variant):
    source, changes = base_instrument(source, benchmark, variant)
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
    let identity = (digestof this).string()
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
