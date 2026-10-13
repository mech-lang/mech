"""Unexecuted exact-text adapter proposal; no CLI, imports, files or processes.

Calling derive is source generation and must occur only after root packet review,
under the actual common benchmark mutex, with authoritative original source pins.
This module never claims compiler acceptance, native coverage or qualification.
Every edit names its original character span and literal replacement; remaining
original text stays byte-for-byte in place. The caller must pin encoded bytes.
"""

DRIVERS = {
    'binarytrees/upstream/binarytrees-julia-3.jl': (
        'isinteractive() || binary_trees(stdout, parse(Int, ARGS[1]))\n',
        'binary_trees(stdout, parse(Int, ARGS[1]))\n'),
    'binarytrees/upstream/binarytrees-julia-4.jl': (
        'isinteractive() || binary_trees(stdout, parse(Int, ARGS[1]))\n',
        'binary_trees(stdout, parse(Int, ARGS[1]))\n'),
    'fannkuchredux/upstream/fannkuchredux-julia-1.jl': (
        'n = parse(Int,ARGS[1])\npf = perf_fannkuch(n)\nprintln(pf[1])\nprintln("Pfannkuchen(", n, ") = ", pf[2])\n',
        'n = parse(Int,ARGS[1])\npf = perf_fannkuch(n)\nprintln(pf[1])\nprintln("Pfannkuchen(", n, ") = ", pf[2])\n'),
    'fannkuchredux/upstream/fannkuchredux-julia-2.jl': (
        'n = parse(Int, ARGS[1])\nfannkuchredux(n)\n',
        'n = parse(Int, ARGS[1])\nfannkuchredux(n)\n'),
    'fasta/upstream/fasta-julia-3.jl': (
        'n = parse(Int,ARGS[1])\nperf_fasta(n)\n',
        'global n = parse(Int,ARGS[1])\nperf_fasta(n)\n'),
    'knucleotide/upstream/knucleotide-julia-1.jl': (
        'perf_k_nucleotide()\n', 'perf_k_nucleotide()\n'),
    'knucleotide/upstream/knucleotide-julia-6.jl': (
        'isinteractive() || main(stdin, stdout)\n', 'main(stdin, stdout)\n'),
    'mandelbrot/upstream/mandelbrot-julia-1.jl': (
        'n = parse(Int,ARGS[1])\nperf_mandelbrot(n)\n',
        'n = parse(Int,ARGS[1])\nperf_mandelbrot(n)\n'),
    'mandelbrot/upstream/mandelbrot-julia-7.jl': (
        'isinteractive() || mandelbrot(stdout, parse(Int, ARGS[1]))\n',
        'mandelbrot(stdout, parse(Int, ARGS[1]))\n'),
    'nbody/upstream/nbody-julia-5.jl': (
        'if !isinteractive()\n'
        '    # The expanded form of Printf.@printf macro takes a significant time to compile,\n'
        '    # accounting for 10% of the total program runtime. Base.Ryu.writefixed(::Float64, ::Int)\n'
        '    # should already be compiled into the default system image.\n'
        '    println(Base.Ryu.writefixed(energy(bodies), 9))\n'
        '    nbody!(bodies, parse(Int, ARGS[1]))\n'
        '    println(Base.Ryu.writefixed(energy(bodies), 9))\nend\n',
        '# Preserve both original energy prints around all runtime steps.\n'
        'println(Base.Ryu.writefixed(energy(bodies), 9))\n'
        'nbody!(bodies, parse(Int, ARGS[1]))\n'
        'println(Base.Ryu.writefixed(energy(bodies), 9))\n'),
    'pidigits/upstream/pidigits-julia-3.jl': (
        'n = parse(Int,ARGS[1])\npidigits(n)\n',
        'n = parse(Int,ARGS[1])\npidigits(n)\n'),
    'pidigits/upstream/pidigits-julia-4.jl': (
        'n = parse(Int,ARGS[1])\npidigits(n)\n',
        'global n = parse(Int,ARGS[1])\npidigits(n)\n'),
    'regexredux/upstream/regexredux-julia-1.jl': (
        'perf_regex_dna()\n', 'perf_regex_dna()\n'),
    'regexredux/upstream/regexredux-julia-3.jl': (
        'isinteractive() || perf_regex_dna(stdin, stdout)\n',
        'perf_regex_dna(stdin, stdout)\n'),
    'revcomp/upstream/revcomp-julia-7.jl': (
        'isinteractive() || perf_revcomp()\n', 'perf_revcomp()\n'),
    'revcomp/upstream/revcomp-julia-9.jl': (
        'isinteractive() || perf_revcomp()\n', 'perf_revcomp()\n'),
    'spectralnorm/upstream/spectralnorm-julia-4.jl': (
        'isinteractive() || println(Base.Ryu.writefixed(main(parse(Int, ARGS[1])), 9))\n',
        'println(Base.Ryu.writefixed(main(parse(Int, ARGS[1])), 9))\n'),
}

SUBSTITUTIONS = (
    ('tHa[Nt]', '<4>'), ('aND|caN|Ha[DS]|WaS', '<3>'),
    ('a[NSt]|BY', '<2>'), ('<[^>]*>', '|'), (r'\|[^|][^|]*\|', '-'),
)
VARIANTS = ('agggtaaa|tttaccct', '[cgt]gggtaaa|tttaccc[acg]',
    'a[act]ggtaaa|tttacc[agt]t', 'ag[act]gtaaa|tttac[agt]ct',
    'agg[act]taaa|ttta[agt]cct', 'aggg[acg]aaa|ttt[cgt]ccct',
    'agggt[cgt]aa|tt[acg]accct', 'agggta[cgt]a|t[acg]taccct',
    'agggtaa[cgt]|[acg]ttaccct')


def function(name, statements):
    return 'function ' + name + '()\n' + ''.join('    ' + line + '\n'
        for line in statements.rstrip('\n').split('\n')) + 'end\n'


def derive(source, relative_path):
    """Return unqualified candidate text and exact edits, never touch a file.

    Initial source hashes/runtime/module pins and mutex ownership are the caller's
    required prerequisite. The pure edit helper does not replace those checks.
    """
    assert relative_path in DRIVERS
    original_driver, runtime_driver = DRIVERS[relative_path]
    # Driver occurrences can also appear in a function declaration; only the
    # reviewed entire terminal suffix is removed, never a generic text search.
    assert source.endswith(original_driver), 'Original terminal driver changed'
    driver_start = len(source) - len(original_driver)
    edits = []
    initializers = []
    appended_declarations = []
    review_obligations = ['native-method-coverage', 'all-runtime-input-work',
                          'complete-output', 'count-all-compiled-app-glue',
                          'fresh-matched-derived-jit', 'serialized-state-review']

    def edit(start, end, replacement, purpose):
        assert 0 <= start < end <= len(source)
        assert all(end <= item['start'] or start >= item['end'] for item in edits)
        edits.append(dict(start=start, end=end, original=source[start:end],
                          replacement=replacement, purpose=purpose))

    def replace_once(old, new, purpose):
        assert source.count(old) == 1, ('Reviewed source anchor changed', old)
        start = source.index(old)
        edit(start, start + len(old), new, purpose)

    if relative_path.startswith('nbody/'):
        start = source.index('const bodies = [\n')
        assert source.count('const bodies = [\n') == 1
        suffix = '\n]\npush!(bodies, init_sun(bodies))\n'
        finish = source.index(suffix, start) + len(suffix)
        literal = source[start:finish]
        initializers.append(literal.replace('const bodies = [\n', 'global bodies = [\n', 1))
        edit(start, finish, 'global bodies::Vector{Body}\n',
             'Fresh exact four-Body literal and original sun construction; typed binding change')
    elif relative_path.startswith('fasta/'):
        for name, julia_type, expression in (
            ('iub2', 'Vector{Float64}', '[0.27, 0.12, 0.12, 0.27, 0.02,0.02, 0.02, 0.02, 0.02, 0.02,0.02, 0.02, 0.02, 0.02, 0.02]'),
            ('homosapiens2', 'Vector{Float64}', '[0.3029549426680, 0.1979883004921,0.1975473066391, 0.3015094502008]'),
            ('state', 'Base.RefValue{Int32}', 'Ref(Int32(42))')):
            old = 'const ' + name + ' = ' + expression + '\n'
            replace_once(old, 'global ' + name + '::' + julia_type + '\n',
                         'Fresh original mutable state; const-to-typed binding change')
            initializers.append('global ' + name + ' = ' + expression + '\n')
        appended_declarations.append('global n::Int\n')
        # Original read-only b literals, alu string and LCG constants stay exact.
    elif relative_path.startswith('revcomp/'):
        block = ('const INIO = stdin\nconst OUTIO = stdout\n\n'
                 'const COMPS = Vector{UInt8}(undef, 256)\n'
                 'for (i, j) in zip("AaCcGgTtUuMmRrWwSsYyKkVvHhDdBbNn",\n'
                 '                  "TTGGCCAAAAKKYYWWSSRRMMBBDDHHVVNN")\n'
                 '    @inbounds COMPS[i % UInt8] = j % UInt8\nend\n')
        replace_once(block, 'global INIO::IO\nglobal OUTIO::IO\nglobal COMPS::Vector{UInt8}\n',
                     'Fresh actual stdio and original table fill; explicit abstract-IO binding change')
        initializers.append(block.replace('const ', 'global '))
        review_obligations.append('actual-regular-file-stdio-subtype-native-coverage')
    elif relative_path.startswith('regexredux/'):
        optimized = relative_path.endswith('-3.jl')
        opening, closing = ('(', ')') if optimized else ('[', ']')
        for name, julia_type, patterns in (
            ('variants', 'NTuple{9,Regex}' if optimized else 'Vector{String}', VARIANTS),
            ('subs', 'NTuple{5,Tuple{Regex,String}}' if optimized else 'Vector{Tuple{Regex,String}}', SUBSTITUTIONS)):
            start_anchor = 'const ' + name + ' = ' + opening + '\n'
            assert source.count(start_anchor) == 1
            start = source.index(start_anchor)
            finish = source.index('\n' + closing + '\n', start) + len('\n' + closing + '\n')
            old = source[start:finish]
            runtime = old.replace('const ', 'global ', 1)
            regex_patterns = patterns if name == 'variants' and optimized else (
                tuple(pattern for pattern, _ in patterns) if name == 'subs' else ())
            for pattern in regex_patterns:
                literal = 'r"' + pattern + '"'
                assert runtime.count(literal) == 1
                runtime = runtime.replace(literal, 'Regex(raw"' + pattern + '")', 1)
            initializers.append(runtime)
            edit(start, finish, 'global ' + name + '::' + julia_type + '\n',
                 'Fresh original collection and Regex handles; explicit typed binding change')
        cleaning = r'r">.*\n|\n"'
        replace_once(cleaning, r'Regex(raw">.*\n|\n")',
                     'Runtime construction of original input-cleaning Regex; same default options')
        review_obligations.append('default-regex-option-pattern-identity-and-vendor-jit-disclosure')
    elif relative_path.endswith('pidigits-julia-4.jl'):
        appended_declarations.append('global n::Int\n')

    edit(driver_start, len(source), function('run_original_driver', runtime_driver),
         'Compiled entry executes original driver with actual runtime ARGS/stdin/stdout')
    derived = source
    for item in sorted(edits, key=lambda item: item['start'], reverse=True):
        assert derived[item['start']:item['end']] == item['original']
        derived = derived[:item['start']] + item['replacement'] + derived[item['end']:]
    derived += '\n' + ''.join(appended_declarations)
    derived += function('initialize_runtime_state', '\n'.join(initializers) if initializers else 'nothing\n')
    if relative_path.endswith('binarytrees-julia-4.jl'):
        # @everywhere definition expansion remains build-time original code.
        # Actual no-added-worker policy and @distributed runtime paths still
        # require native observation; this helper never converts algorithms.
        review_obligations += ['no-added-distributed-workers', 'no-runtime-eval-or-interpreter',
                               'everywhere-defines-Main-not-containing-package-module']
    if relative_path.startswith('binarytrees/'):
        review_obligations += ['unchanged-scored-leaf-interior-allocation', 'actual-retained-long-graph']
    return dict(source=derived, original_source=relative_path, edits=sorted(edits, key=lambda item: item['start']),
                appended_declarations=appended_declarations, runtime_initializers=initializers,
                review_obligations=review_obligations, matched_derived_jit_required=True,
                module_context_qualified=False,
                original_jit_samples_preserved=True, source_transformation_only=True,
                compiler_acceptance=False, runtime_aot_proven=False, chart_admission=False)
