"""Exact additive diagnostic instrumentation of the declared production sources.

Diagnostics use POSIX write and pthread identity only. No workload is delegated
to native support; every source operation remains in its original order.
"""
import re

TRACE = '''use @write[I64](fd: I32, buffer: Pointer[U8] tag, count: USize)
use @pthread_self[USize]()
use @exit[None](status: I32)

primitive Trace
  fun apply(message: String) =>
    let line = message + "\\n"
    var offset: USize = 0
    while offset < line.size() do
      let written = @write(2, line.cpointer(offset), line.size() - offset)
      if written <= 0 then @exit(1) end
      offset = offset + written.usize()
    end
'''


def instrument(source, benchmark, variant):
    changes = []
    def replace(old, new):
        nonlocal source
        assert source.count(old) == 1, (benchmark, variant, 'nonunique hook', old, source.count(old))
        source = source.replace(old, new)
        changes.append(dict(old=old, new=new))

    if benchmark == 'binarytrees':
        replace('class Node\n', 'class Node\n  let _trace_depth: USize\n  let _trace_final: String\n')
        replace('  new create(depth: USize) =>\n', '  new create(depth: USize) =>\n    _trace_depth = depth\n    _trace_final = "FINAL\\t" + depth.string() + "\\n"\n    Trace("NODE\\t" + depth.string())\n')
        replace('  fun check(): U64 =>\n', '  fun _final() =>\n    let written = @write(2, _trace_final.cpointer(), _trace_final.size())\n    if written != _trace_final.size().i64() then @exit(1) end\n  fun check(): U64 =>\n    Trace("CHECK\\t" + _trace_depth.string())\n')
        replace('      let tree = Node(_depth)\n', '      Trace("TREE\\t" + _depth.string())\n      let tree = Node(_depth)\n')
        replace('    _long = Node(_maximum)\n', '    Trace("LONG_NEW\\t" + _maximum.string())\n    _long = Node(_maximum)\n')
        replace('        _env.out.print("long lived tree', '        Trace("LONG_CHECK\\t" + _maximum.string())\n        _env.out.print("long lived tree')
    elif benchmark == 'knucleotide':
        if variant == 'compact':
            replace('      _counts(word) = _counts.get_or_else(word, 0) + 1\n', '      _counts(word) = _counts.get_or_else(word, 0) + 1\n      Trace("WINDOW\\t" + _width.string() + "\\t" + _index.string() + "\\t" + word)\n')
            key = 'word'
        else:
            replace('          _counts(_key) = _counts.get_or_else(_key, 0) + 1\n', '          _counts(_key) = _counts.get_or_else(_key, 0) + 1\n          Trace("WINDOW\\t" + _width.string() + "\\t" + ((_index + 1) - _width).string() + "\\t" + Packed.decode(_key, _width))\n')
            key = 'Packed.decode(word, _width)'
        replace('      _parent.done(_slot, consume counts)\n', '      Trace("MAP_DONE\\t" + _width.string() + "\\t" + counts.size().string())\n      for (word, count) in counts.pairs() do\n        Trace("MAP\\t" + _width.string() + "\\t" + ' + key + ' + "\\t" + count.string())\n      end\n      _parent.done(_slot, consume counts)\n')
        replace('      _maps(slot)? = consume counts\n', '      _maps(slot)? = consume counts\n      Trace("MAP_PARENT\\t" + _lengths(slot)?.string() + "\\t" + _maps(slot)?.size().string())\n')
        replace('    for index in Range[USize](0, 2) do\n', '    for trace_slot in Range[USize](0, 7) do\n      for (trace_word, trace_count) in _maps(trace_slot)?.pairs() do\n        Trace("PARENT_MAP\\t" + _lengths(trace_slot)?.string() + "\\t" + ' + ('trace_word' if variant == 'compact' else 'Packed.decode(trace_word, _lengths(trace_slot)?)') + ' + "\\t" + trace_count.string())\n      end\n    end\n    for index in Range[USize](0, 2) do\n')
    elif benchmark == 'pidigits':
        replace('      let digit4 = numerator4 / _t\n', '      let digit4 = numerator4 / _t\n      Trace("PI\\t" + _q.string() + "\\t" + _r.string() + "\\t" + _t.string() + "\\t" + _k.string() + "\\t" + numerator3.string() + "\\t" + numerator4.string() + "\\t" + digit3.string() + "\\t" + digit4.string())\n')
    else:
        raise AssertionError(benchmark)
    # Actor behavior entry records scheduler identities only (not ASIO callbacks).
    before = source
    source = re.sub(r'(  be [^\n]+ =>\n)', r'\1    Trace("THREAD\\t" + @pthread_self().string())\n', source)
    changes.append(dict(kind='scheduler-identity-hooks', before=before, after=source))
    # Keep all package/FFI use directives ahead of the first type declaration.
    # The production import block is preserved byte-for-byte; diagnostic
    # declarations are inserted only after both original and added imports.
    rows = source.splitlines(keepends=True)
    boundary = 0
    while boundary < len(rows) and (rows[boundary].startswith('use ') or not rows[boundary].strip()):
        boundary += 1
    original_imports = ''.join(rows[:boundary])
    diagnostic_imports, diagnostic_definition = TRACE.split('\n\n', 1)
    changes.append(dict(kind='diagnostic-use-directive-order', original_imports=original_imports,
                        diagnostic_imports=diagnostic_imports, first_type='Trace'))
    return original_imports + diagnostic_imports + '\n\n' + diagnostic_definition + '\n' + ''.join(rows[boundary:]), changes
