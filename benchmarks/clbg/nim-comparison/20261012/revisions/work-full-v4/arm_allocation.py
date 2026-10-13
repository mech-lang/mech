"""Conservative ARM control-flow allocation proof, with one reviewed TLS guard."""
import hashlib
import re

REVIEWED_EXCEPTION_SYMBOLS = {'make__compact_u7', 'make__optimized_u17'}
CONTROL = re.compile(r'^(?:b(?:\.[a-z]+)?|cb[a-z]*|tb[a-z]*|br[a-z]*|ret[a-z]*)$')


def instructions(body):
    result = []
    for line in body.splitlines():
        if not line.strip(): continue
        match = re.fullmatch(r'([0-9a-fA-F]+)\s+([a-z][a-z0-9.]*)\s*(.*?)\s*', line)
        assert match, ('Unknown scored ARM instruction rendering', line)
        arguments = match[3].split(';', 1)[0].strip()
        result.append((int(match[1], 16), match[2], arguments))
    assert result and len({value[0] for value in result}) == len(result)
    assert [value[0] for value in result] == sorted(value[0] for value in result)
    return result


def address(arguments):
    target = arguments.rsplit(',', 1)[-1].strip()
    assert re.fullmatch(r'(?:0x)?[0-9a-fA-F]+', target), ('Unresolved control target', arguments)
    return int(target, 16)


def source_exception_preamble(source):
    assert source is not None
    offset = source.index('nimNewObj(')
    before = source[:offset]
    compact = re.sub(r'\s+', '', before)
    pattern = (r'\{(?P<type>[A-Za-z_]\w*)\*result;NIM_BOOL\*nimErr_;\{'
               r'nimErr_=nimErrorFlag\(\);result=NIM_NIL;eqdestroy___\w+\(result\);'
               r'if\(NIM_UNLIKELY\(\*nimErr_\)\)\)gotoBeforeRet_;result=\((?P=type)\*\)')
    assert re.fullmatch(pattern, compact), 'Preallocation generated C is not the exact error-flag/NIL return preamble'
    assert source.count('nimErr_ = nimErrorFlag();') == 1
    assert re.search(r'BeforeRet_:\s*;\s*return result;', source), 'Error path must return the initialized NIL result'
    return dict(status='exact-generated-C-error-flag-and-NIL-preamble',
                generated_creator_sha256=hashlib.sha256(source.encode()).hexdigest())


def null_return(code, index):
    zero = set(); visited = []
    for _, operation, arguments in code[index:index + 12]:
        visited.append(operation)
        if operation == 'mov':
            target, value = [part.strip() for part in arguments.split(',')]
            assert re.fullmatch(r'x\d+', target)
            if value in ('#0', '#0x0', 'xzr') or value in zero: zero.add(target)
            else: raise AssertionError('Error return contains a nonzero or unproved value')
        elif operation == 'ldp':
            first, second, _ = arguments.split(',', 2)
            registers = {first.strip(), second.strip()}
            assert 'x0' not in registers
            zero -= registers
        elif operation == 'add':
            assert re.fullmatch(r'sp,\s*sp,\s*#(?:0x)?[0-9a-fA-F]+', arguments)
        elif operation == 'nop':
            assert arguments == ''
        elif operation == 'ret':
            assert arguments in ('', 'x30') and 'x0' in zero
            return visited
        else:
            raise AssertionError('Error target is not a bounded NULL-only epilogue')
    raise AssertionError('Error target has no bounded NULL return')


def exception_guard(code, allocation_index, name, source):
    prefix = code[:allocation_index]
    controls = [index for index, (_, operation, _) in enumerate(prefix) if CONTROL.fullmatch(operation)]
    if not controls:
        assert not any(operation.startswith('bl') for _, operation, _ in prefix), 'Unknown preallocation call'
        return None
    assert name in REVIEWED_EXCEPTION_SYMBOLS, 'This scored creator needs its own actual exception-path review'
    source_proof = source_exception_preamble(source)
    assert len(controls) == 1
    branch = controls[0]
    assert branch >= 5 and prefix[branch][1] == 'tbnz'
    sequence = prefix[branch - 5:branch + 1]
    assert [item[1] for item in sequence] == ['adrp', 'add', 'ldr', 'blr', 'ldrb', 'tbnz']
    assert re.fullmatch(r'x0,\s*(?:0x)?[0-9a-fA-F]+', sequence[0][2])
    assert re.fullmatch(r'x0,\s*x0,\s*#(?:0x)?[0-9a-fA-F]+', sequence[1][2])
    loaded = re.fullmatch(r'(x\d+),\s*\[x0\]', sequence[2][2])
    assert loaded and sequence[3][2] == loaded[1], 'Indirect TLS call must use its exact loaded resolver'
    flag = re.fullmatch(r'(w\d+),\s*\[x0\]', sequence[4][2])
    tested = re.fullmatch(r'(w\d+),\s*#(?:0x)?0,\s*((?:0x)?[0-9a-fA-F]+)', sequence[5][2])
    assert flag and tested and flag[1] == tested[1], 'Only the returned TLS Boolean byte may control this exception edge'
    assert [index for index, (_, operation, _) in enumerate(prefix) if operation.startswith('bl')] == [branch - 2]
    assert all(operation in {'stp', 'add', 'mov', 'adrp', 'ldr', 'blr', 'ldrb', 'tbnz'} for _, operation, _ in prefix)
    for _, operation, arguments in prefix:
        if operation == 'stp':
            assert re.fullmatch(r'x\d+,\s*x\d+,\s*\[sp(?:,\s*#-?(?:0x)?[0-9a-fA-F]+)?\]!?', arguments), 'Only stack saves may write before the TLS error test'
    target = address(sequence[5][2]); positions = {item[0]: index for index, item in enumerate(code)}
    assert target in positions and positions[target] > allocation_index
    epilogue = null_return(code, positions[target])
    return dict(status='matched-reviewed-runtime-exception-propagation', branch_address=prefix[branch][0],
        exception_target=target, literal_NULL_only_epilogue=epilogue,
        generated_C_binding=source_proof, normal_entry_condition='nimErrorFlag is false; no pending runtime exception')


def allocation_dominance(body, name, source):
    code = instructions(body)
    allocation_indices = [index for index, (_, operation, arguments) in enumerate(code)
                          if operation == 'bl' and arguments == '_nimNewObj']
    assert len(allocation_indices) == 1
    allocator = allocation_indices[0]
    exception = exception_guard(code, allocator, name, source)
    positions = {value[0]: index for index, value in enumerate(code)}
    pending = [(0, False)]; seen = set(); returns = set()
    while pending:
        index, allocated = pending.pop()
        if (index, allocated) in seen: continue
        assert 0 <= index < len(code), 'Creator control falls out of its function'
        seen.add((index, allocated))
        location, operation, arguments = code[index]
        if index == allocator: allocated = True
        if operation == 'ret':
            assert arguments in ('', 'x30') and allocated, 'A non-exception return bypasses actual allocation'
            returns.add(location); continue
        if operation.startswith(('br', 'ret')):
            raise AssertionError('Unknown indirect control/return requires actual review')
        if CONTROL.fullmatch(operation):
            if exception and location == exception['branch_address']:
                pending.append((index + 1, allocated)); continue
            target = address(arguments)
            assert target in positions, 'A control edge leaves the exact scored creator'
            pending.append((positions[target], allocated))
            if operation == 'b': continue
            assert operation.startswith('b.') or operation in ('cbz', 'cbnz', 'tbz', 'tbnz'), 'Unknown conditional control'
        elif operation.startswith('bl'):
            assert operation in ('bl', 'blr'), 'Unknown call rendering'
            if operation == 'blr':
                assert exception and index < allocator, 'Unresolved indirect call requires actual review'
        pending.append((index + 1, allocated))
    assert returns
    return dict(status='passed', allocation_before_all_nonexception_returns=True,
        allocation_before_workload_or_leaf_control=True, nonexception_return_addresses=sorted(returns),
        preallocation_exception_guard=exception, unconditional_allocation_before_control=exception is None)
