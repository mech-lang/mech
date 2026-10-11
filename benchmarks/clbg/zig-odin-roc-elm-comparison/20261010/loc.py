"""Noncomment LOC, with independent packed statements counted separately."""
import re

def normalize(source, language):
    # Mask strings and remove comments while preserving line positions.
    token = re.compile(r'"(?:\\.|[^"\\])*"|\x27(?:\\.|[^\x27\\])*\x27|/\*.*?\*/|//[^\n]*|--[^\n]*|#[^\n]*', re.S)
    def mask(match):
        text = match[0]
        if text.startswith(('"', "'")):
            return 'S' + '\n' * text.count('\n')
        is_comment = text.startswith(('/*','//')) if language in ('zig','odin') else text.startswith('--') if language == 'elm' else text.startswith('#')
        return '\n' * text.count('\n') if is_comment else text
    cleaned = token.sub(mask, source)
    physical = sum(bool(line.strip()) for line in cleaned.splitlines())
    expansions = []
    for number, line in enumerate(cleaned.splitlines(),1):
        if not line.strip(): continue
        count = 1
        if language == 'zig':
            # Semicolons in a for/while control header are structural.
            parts = re.sub(r'\([^()]*(?:;[^()]*)+\)', '()', line)
            count = max(count, parts.count(';'))
        if language in ('zig','odin'):
            statements = re.sub(r'\.\{[^{}]*\}', 'LITERAL', line)
            assignments = len(re.findall(r'(?<![=!<>])(?:[+*/-]?=(?!=)|:=)', statements))
            commands = len(re.findall(r'\b(?:return|break|continue)\b', line))
            if not re.search(r'\bfor\b.*?;', line):
                count = max(count, assignments + commands)
            else:
                count = max(count, commands)
        if count > 1:
            expansions.append(dict(line=number,extra_lines=count-1,reason='Independent statements or initialized bindings packed on one source line.'))
    return dict(physical_loc=physical,normalized_loc=physical+sum(e['extra_lines'] for e in expansions),expansions=expansions)

def fixtures():
    assert normalize('x = 1; y = 2;', 'zig')['normalized_loc'] == 2
    assert normalize('if x { a = 1 } else { a = 2 }', 'odin')['normalized_loc'] == 2
    assert normalize('for i := 0; i < n; i += 1 { f(i) }', 'odin')['normalized_loc'] == 1
    assert normalize('f = |x| x + 1 # comment', 'roc')['normalized_loc'] == 1
    assert normalize('f x = x + 1 -- comment', 'elm')['normalized_loc'] == 1
    assert normalize('x = "// ; a = 1";', 'zig')['normalized_loc'] == 1
    assert normalize('node.* = .{ .x = x, .y = y };', 'zig')['normalized_loc'] == 1
