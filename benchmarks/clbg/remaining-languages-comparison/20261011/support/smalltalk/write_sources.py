"""Explicit Smalltalk applications; generated .st is the complete counted source."""
from pathlib import Path
import json

D = Path(__file__).resolve().parents[2]
SOURCES = D/'sources/smalltalk'
HEADER = '''"Computer Language Benchmarks Game. Local native Pharo adaptations, 2026."
Object subclass: #BenchmarksGame
    instanceVariableNames: ''
    classVariableNames: ''
    poolDictionaries: ''
    category: 'Benchmarks-Game'!
!BenchmarksGame class methodsFor: 'application'!
'''


def app(name, methods, notes, extra=''):
    benchmark = name.split('-')[0]
    path = SOURCES/f'{name}.st'
    path.write_text(extra+HEADER+methods+'! !\n')
    return dict(id=name,key=f'{benchmark}:{name}',benchmark=benchmark,language='smalltalk',source=str(path),
                source_url=f'https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/{benchmark}.html',
                local_adaptation=True,adaptations=notes,profile_candidates=['fastest','smallest'])


def main():
    rows=[]
    rows.append(app('binarytrees-smalltalk-arrays',r'''
tree: depth
    ^depth = 0
        ifTrue: [Array with: nil with: nil]
        ifFalse: [Array with: (self tree: depth - 1) with: (self tree: depth - 1)]!
check: tree
    ^tree first isNil
        ifTrue: [1]
        ifFalse: [1 + (self check: tree first) + (self check: tree second)]!
do: n
    | maximum long iterations check out |
    maximum := 6 max: n.
    out := Stdio stdout.
    check := self check: (self tree: maximum + 1).
    out nextPutAll: 'stretch tree of depth ';
        print: maximum + 1;
        nextPut: Character tab;
        nextPutAll: ' check: ';
        print: check;
        lf.
    long := self tree: maximum.
    4 to: maximum by: 2 do: [:depth |
        iterations := 1 bitShift: maximum - depth + 4.
        check := 0.
        iterations timesRepeat: [check := check + (self check: (self tree: depth))].
        out print: iterations;
            nextPutAll: String tab, ' trees of depth ';
            print: depth;
            nextPutAll: String tab, ' check: ';
            print: check;
            lf].
    out nextPutAll: 'long lived tree of depth ';
        print: maximum;
        nextPutAll: String tab, ' check: ';
        print: (self check: long);
        lf
''',['Two-slot Array records allocate a distinct equal-size object for every interior and leaf; recursive construction/checks retained. The long-lived root is allocated before temporary iterations and checked after them.']))

    rows.append(app('pidigits-smalltalk-compact',r'''
do: n
    | q r t k digit emitted out |
    q := 1.
    r := 0.
    t := 1.
    k := 0.
    emitted := 0.
    out := Stdio stdout.
    [emitted < n] whileTrue: [
        k := k + 1.
        r := (r + (q * 2)) * (k * 2 + 1).
        q := q * k.
        t := t * (k * 2 + 1).
        q <= r ifTrue: [
            digit := (q * 3 + r) // t.
            digit = ((q * 4 + r) // t) ifTrue: [
                out nextPut: (Character digitValue: digit).
                emitted := emitted + 1.
                r := (r - (t * digit)) * 10.
                q := q * 10.
                (emitted \\ 10 = 0 or: [emitted = n]) ifTrue: [
                    emitted \\ 10 = 0 ifFalse: [
                        1 to: 10 - (emitted \\ 10) do: [:unused | out nextPut: Character space]].
                    out nextPut: Character tab;
                        nextPut: $:;
                        print: emitted;
                        lf]]]
    ]
''',['The full arbitrary precision streaming pi spigot is written in one Smalltalk method, using standard Integer operations; no native benchmark routine or precomputed digits. Correct partial output-line padding.']))

    rows.append(app('spectralnorm-smalltalk-recurrence',r'''
multiply: vector transpose: transposed
    | result n sum denominator step |
    n := vector size.
    result := Float64Array new: n.
    0 to: n - 1 do: [:i |
        denominator := i * (i + 1) // 2 + (transposed ifTrue: [1] ifFalse: [i + 1]).
        step := i + (transposed ifTrue: [2] ifFalse: [1]).
        sum := 0.0d0.
        1 to: n do: [:j |
            sum := sum + ((vector at: j) / denominator).
            denominator := denominator + step.
            step := step + 1].
        result at: i + 1 put: sum].
    ^result!
ata: vector
    ^self multiply: (self multiply: vector transpose: false) transpose: true!
do: n
    | u v uv vv |
    u := Float64Array new: n withAll: 1.0d0.
    10 timesRepeat: [
        v := self ata: u.
        u := self ata: v].
    uv := 0.0d0.
    vv := 0.0d0.
    1 to: n do: [:i |
        uv := uv + ((u at: i) * (v at: i)).
        vv := vv + ((v at: i) * (v at: i))].
    Stdio stdout nextPutAll: ((uv / vv) sqrt printShowingDecimalPlaces: 9);
        lf
''',['Use standard Float64Array and exact integer recurrences for the implicit matrix denominator. Retain all 40 dense matrix-vector passes with independent dimension n; no cached matrix or eigenvalue.']))

    rows.append(app('fannkuchredux-smalltalk-reuse',r'''
do: n
    | perm count scratch maximum checksum index flips first left right temp r rotated more |
    perm := (1 to: n) asArray.
    count := Array new: n withAll: 0.
    scratch := Array new: n.
    maximum := 0.
    checksum := 0.
    index := 0.
    more := true.
    [more] whileTrue: [
        flips := 0.
        (perm at: 1) = 1 ifFalse: [
            scratch replaceFrom: 1 to: n with: perm startingAt: 1.
            [(first := scratch at: 1) = 1] whileFalse: [
                left := 1.
                right := first.
                [left < right] whileTrue: [
                    temp := scratch at: left.
                    scratch at: left put: (scratch at: right).
                    scratch at: right put: temp.
                    left := left + 1.
                    right := right - 1].
                flips := flips + 1]].
        checksum := index even ifTrue: [checksum + flips] ifFalse: [checksum - flips].
        maximum := maximum max: flips.
        index := index + 1.
        r := 2.
        rotated := false.
        [r <= n and: [rotated not]] whileTrue: [
            temp := perm at: 1.
            1 to: r - 1 do: [:i | perm at: i put: (perm at: i + 1)].
            perm at: r put: temp.
            count at: r put: ((count at: r) + 1) \\ r.
            (count at: r) = 0 ifTrue: [r := r + 1] ifFalse: [rotated := true]].
        more := rotated].
    Stdio stdout print: checksum;
        lf;
        nextPutAll: 'Pfannkuchen(';
        print: n;
        nextPutAll: ') = ';
        print: maximum;
        lf
''',['Enumerate all n! permutations in canonical prefix-rotation order and preserve alternating checksum. Reuse a scratch array through the standard bulk copy primitive rather than allocate one array per permutation; no input-specific factorial truncation.']))

    nbody=r'''
bodies
    | b mass |
    mass := 4.0d0 * Float pi * Float pi.
    b := #(
        #(0.0d0 0.0d0 0.0d0 0.0d0 0.0d0 0.0d0 1.0d0)
        #(4.84143144246472090d0 -1.16032004402742839d0 -1.03622044471123109d-1 1.66007664274403694d-3 7.69901118419740425d-3 -6.90460016972063023d-5 9.54791938424326609d-4)
        #(8.34336671824457987d0 4.12479856412430479d0 -4.03523417114321381d-1 -2.76742510726862411d-3 4.99852801234917238d-3 2.30417297573763929d-5 2.85885980666130812d-4)
        #(1.28943695621391310d1 -1.51111514016986312d1 -2.23307578892655734d-1 2.96460137564761618d-3 2.37847173959480950d-3 -2.96589568540237556d-5 4.36624404335156298d-5)
        #(1.53796971148509165d1 -2.59193146099879641d1 1.79258772950371181d-1 2.68067772490389322d-3 1.62824170038242295d-3 -9.51592254519715870d-5 5.15138902046611451d-5)
    ) collect: [:row | Float64Array withAll: row].
    b do: [:row |
        4 to: 6 do: [:k | row at: k put: (row at: k) * 365.24d0].
        row at: 7 put: (row at: 7) * mass].
    4 to: 6 do: [:k |
        b first at: k put: (b sum: [:row | (row at: k) * (row at: 7)]) negated / mass].
    ^b!
energy: bodies
    | total a b dx dy dz |
    total := 0.0d0.
    1 to: bodies size do: [:i |
        a := bodies at: i.
        total := total + (0.5d0 * (a at: 7) * ((a at: 4) squared + (a at: 5) squared + (a at: 6) squared)).
        i + 1 to: bodies size do: [:j |
            b := bodies at: j.
            dx := (a at: 1) - (b at: 1).
            dy := (a at: 2) - (b at: 2).
            dz := (a at: 3) - (b at: 3).
            total := total - ((a at: 7) * (b at: 7) / (dx * dx + (dy * dy) + (dz * dz)) sqrt)]].
    ^total!
advance: bodies
    | a b dx dy dz distance magnitude am bm |
    1 to: bodies size do: [:i |
        a := bodies at: i.
        i + 1 to: bodies size do: [:j |
            b := bodies at: j.
            dx := (a at: 1) - (b at: 1).
            dy := (a at: 2) - (b at: 2).
            dz := (a at: 3) - (b at: 3).
            distance := (dx * dx + (dy * dy) + (dz * dz)) sqrt.
            magnitude := 0.01d0 / (distance * distance * distance).
            am := (a at: 7) * magnitude.
            bm := (b at: 7) * magnitude.
            a at: 4 put: (a at: 4) - (dx * bm).
            a at: 5 put: (a at: 5) - (dy * bm).
            a at: 6 put: (a at: 6) - (dz * bm).
            b at: 4 put: (b at: 4) + (dx * am).
            b at: 5 put: (b at: 5) + (dy * am).
            b at: 6 put: (b at: 6) + (dz * am)]].
    bodies do: [:body |
        1 to: 3 do: [:k | body at: k put: (body at: k) + (0.01d0 * (body at: k + 3))]]!
do: n
    | b |
    b := self bodies.
    Stdio stdout nextPutAll: ((self energy: b) printShowingDecimalPlaces: 9);
        lf.
    n timesRepeat: [self advance: b].
    Stdio stdout nextPutAll: ((self energy: b) printShowingDecimalPlaces: 9);
        lf
'''
    rows.append(app('nbody-smalltalk-floatarrays',nbody,['Complete five-body initialization, momentum offset, exact binary64 square roots/divisions and all ten pair updates per step in Smalltalk; uses ordinary Float64Array records and explicit general iteration, without C/SIMD benchmark kernels.']))

    fasta=r'''
alu
    ^'GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG',
      'GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA',
      'CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT',
      'ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA',
      'GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG',
      'AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC',
      'AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA'!
random: length alphabet: alphabet probabilities: probabilities seed: seed to: out
    | cumulative total bytes buffer amount r lo hi mid remaining state |
    remaining := length.
    state := seed.
    total := 0.0d0.
    cumulative := probabilities collect: [:p | total := total + p].
    bytes := alphabet asByteArray.
    buffer := ByteArray new: 61.
    [remaining > 0] whileTrue: [
        amount := remaining min: 60.
        1 to: amount do: [:i |
            state := (state * 3877 + 29573) \\ 139968.
            r := state asFloat / 139968.0d0.
            lo := 1.
            hi := cumulative size.
            [lo < hi] whileTrue: [
                mid := (lo + hi) // 2.
                r < (cumulative at: mid)
                    ifTrue: [hi := mid]
                    ifFalse: [lo := mid + 1]].
            buffer at: i put: (bytes at: lo)].
        buffer at: amount + 1 put: 10.
        out next: amount + 1 putAll: buffer startingAt: 1.
        remaining := remaining - amount].
    ^state!
do: n
    | out alu length index buffer amount seed |
    out := ZnBufferedWriteStream on: Stdio stdout binary.
    out nextPutAll: '>ONE Homo sapiens alu' asByteArray;
        nextPut: 10.
    alu := self alu asByteArray.
    length := n * 2.
    index := 1.
    buffer := ByteArray new: 61.
    [length > 0] whileTrue: [
        amount := length min: 60.
        1 to: amount do: [:i |
            buffer at: i put: (alu at: index).
            index := index \\ alu size + 1].
        buffer at: amount + 1 put: 10.
        out next: amount + 1 putAll: buffer startingAt: 1.
        length := length - amount].
    out nextPutAll: '>TWO IUB ambiguity codes' asByteArray;
        nextPut: 10.
    seed := self random: n * 3 alphabet: 'acgtBDHKMNRSVWY'
        probabilities: #(0.27d0 0.12d0 0.12d0 0.27d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0)
        seed: 42 to: out.
    out nextPutAll: '>THREE Homo sapiens frequency' asByteArray;
        nextPut: 10.
    self random: n * 5 alphabet: 'acgt'
        probabilities: #(0.3029549426680d0 0.1979883004921d0 0.1975473066391d0 0.3015094502008d0)
        seed: seed to: out.
    out flush
'''
    rows.append(app('fasta-smalltalk-buffered',fasta,['Shared LCG state is advanced once for each of 8*n random symbols. Binary64 seed division and unscaled cumulative probabilities retained; general binary search through each CDF and reusable output bytes reduce polymorphic per-character stream overhead. Complete FASTA output emitted.']))

    read=r'''
readThree
    | input buffer active line |
    input := ZnCharacterReadStream on: (ZnBufferedReadStream on: Stdio stdin).
    buffer := WriteStream on: (String new: 1024).
    active := false.
    [input atEnd] whileFalse: [
        line := input nextLine.
        (line beginsWith: '>')
            ifTrue: [
                active ifTrue: [^buffer contents asUppercase].
                active := line beginsWith: '>THREE']
            ifFalse: [
                (active and: [line isNotEmpty and: [line first ~= $;]]) ifTrue: [buffer nextPutAll: line]]].
    ^buffer contents asUppercase!
'''
    counts=r'''
countsIn: sequence length: length
    | counts mask code association |
    counts := Dictionary new.
    mask := (1 bitShift: length * 2) - 1.
    code := 0.
    1 to: sequence size do: [:i |
        code := ((code bitShift: 2) bitOr: (sequence at: i)) bitAnd: mask.
        i >= length ifTrue: [
            association := counts associationAt: code ifAbsent: [nil].
            association
                ifNil: [counts at: code put: 1]
                ifNotNil: [association value: association value + 1]]].
    ^counts!
keyOf: string
    ^string inject: 0 into: [:key :char | (key bitShift: 2) bitOr: ('ACGT' indexOf: char) - 1]!
decode: key length: length
    | text remaining |
    text := String new: length.
    remaining := key.
    length to: 1 by: -1 do: [:i |
        text at: i put: ('ACGT' at: (remaining bitAnd: 3) + 1).
        remaining := remaining bitShift: -2].
    ^text!
do: n
    | string sequence tables out rows total |
    string := self readThree.
    sequence := ByteArray new: string size.
    1 to: string size do: [:i |
        ('ACGT' includes: (string at: i)) ifFalse: [self error: 'Expected A, C, G or T DNA'].
        sequence at: i put: ('ACGT' indexOf: (string at: i)) - 1].
    tables := Dictionary new.
    #(1 2 3 4 6 12 18) do: [:length |
        tables at: length put: (self countsIn: sequence length: length)].
    out := Stdio stdout.
    #(1 2) do: [:length |
        rows := (tables at: length) associations asSortedCollection: [:a :b |
            a value = b value ifTrue: [a key < b key] ifFalse: [a value > b value]].
        total := sequence size - length + 1.
        rows do: [:row |
            out nextPutAll: (self decode: row key length: length);
                nextPut: Character space;
                nextPutAll: (row value * 100.0d0 / total printShowingDecimalPlaces: 3);
                lf].
        out lf].
    #('GGT' 'GGTA' 'GGTATT' 'GGTATTTTAATT' 'GGTATTTTAATTTATAGT') do: [:query |
        out print: ((tables at: query size) at: (self keyOf: query) ifAbsent: [0]);
            nextPut: Character tab;
            nextPutAll: query;
            lf]
'''
    rows.append(app('knucleotide-smalltalk-packed',read+counts,['Exact two-bit ACGT keys up to 36 bits use standard native SmallInteger, seven ordinary growable Dictionary maps and every overlapping window. The standard Association API updates an existing value after one lookup; no five-motif-only scan or fixed input table. General FASTA record parsing stops at the next header and handles comments/CRLF.']))

    patterns=r'''
readInput
    | input output block |
    input := Stdio stdin binary.
    output := WriteStream on: (ByteArray new: 65536).
    [(block := input next: 65536) isNotEmpty] whileTrue: [output nextPutAll: block].
    ^output contents!
patterns
    ^#('agggtaaa|tttaccct'
        '[cgt]gggtaaa|tttaccc[acg]'
        'a[act]ggtaaa|tttacc[agt]t'
        'ag[act]gtaaa|tttac[agt]ct'
        'agg[act]taaa|ttta[agt]cct'
        'aggg[acg]aaa|ttt[cgt]ccct'
        'agggt[cgt]aa|tt[acg]accct'
        'agggta[cgt]a|t[acg]taccct'
        'agggtaa[cgt]|[acg]ttaccct')!
substitutions
    ^#(#('tHa[Nt]' '<4>')
        #('aND|caN|Ha[DS]|WaS' '<3>')
        #('a[NSt]|BY' '<2>')
        #('<[^>]*>' '|')
        #('\|[^|][^|]*\|' '-'))!
'''
    rows.append(app('regexredux-smalltalk-regex',patterns+r'''
do: n
    | raw sequence originalSize count out |
    raw := self readInput asString.
    sequence := raw copyWithRegex: ('>[^', String lf, ']*', String lf, '|', String lf) matchesReplacedWith: ''.
    originalSize := sequence size.
    out := Stdio stdout.
    self patterns do: [:pattern |
        count := 0.
        pattern asRegex matchesIn: sequence do: [:match | count := count + 1].
        out nextPutAll: pattern;
            nextPut: Character space;
            print: count;
            lf].
    self substitutions do: [:substitution |
        sequence := sequence copyWithRegex: substitution first matchesReplacedWith: substitution second].
    out lf;
        print: raw size;
        lf;
        print: originalSize;
        lf;
        print: sequence size;
        lf
''',['Correct the upstream obsolete regex-DNA source: remove literal LF FASTA lines, count nine nonoverlapping patterns, perform all five ordered global substitutions through standard Pharo Regex. No specialized recognizer or benchmark executor.']))

    pcre=r'''
Object subclass: #BenchmarkPCRE
    instanceVariableNames: 'code data vector'
    classVariableNames: ''
    poolDictionaries: ''
    category: 'Benchmarks-Game'!
!BenchmarkPCRE class methodsFor: 'general PCRE2 bindings'!
compilePattern: pattern length: length error: error offset: offset
    ^self ffiCall: #(void * pcre2_compile_8(char *pattern, ulong length, uint 0, void *error, void *offset, void *0))
        module: '/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib'!
jit: compiled
    ^self ffiCall: #(int pcre2_jit_compile_8(void *compiled, uint 1))
        module: '/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib'!
newData: compiled
    ^self ffiCall: #(void * pcre2_match_data_create_from_pattern_8(void *compiled, void *0))
        module: '/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib'!
vector: matchData
    ^self ffiCall: #(void * pcre2_get_ovector_pointer_8(void *matchData))
        module: '/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib'!
match: compiled subject: subject size: size start: start data: matchData
    ^self ffiCall: #(int pcre2_match_8(void *compiled, void *subject, ulong size, ulong start, uint 0, void *matchData, void *0))
        module: '/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib'!
substitute: compiled subject: subject size: size data: matchData replacement: replacement length: replacementLength output: output capacity: capacity
    ^self ffiCall: #(int pcre2_substitute_8(void *compiled, void *subject, ulong size, ulong 0, uint 4352, void *matchData, void *0, char *replacement, ulong replacementLength, void *output, void *capacity))
        module: '/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib'!
freeCode: compiled
    ^self ffiCall: #(void pcre2_code_free_8(void *compiled))
        module: '/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib'!
freeData: matchData
    ^self ffiCall: #(void pcre2_match_data_free_8(void *matchData))
        module: '/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib'!
pattern: pattern
    ^self new initialize: pattern! !
!BenchmarkPCRE methodsFor: 'general regular expressions'!
initialize: pattern
    | error offset |
    error := ByteArray new: 4.
    offset := ByteArray new: 8.
    code := self class compilePattern: pattern length: pattern size error: error offset: offset.
    code isNull ifTrue: [self error: 'PCRE2 compile failed'].
    self class jit: code.
    data := self class newData: code.
    data isNull ifTrue: [self error: 'PCRE2 match data allocation failed'].
    vector := self class vector: data!
close
    self class freeData: data.
    self class freeCode: code!
count: subject size: size
    | start matches result next |
    start := 0.
    matches := 0.
    [start <= size] whileTrue: [
        result := self class match: code subject: subject size: size start: start data: data.
        result = -1 ifTrue: [^matches].
        result < 0 ifTrue: [self error: 'PCRE2 match failed'].
        matches := matches + 1.
        next := vector unsignedLongLongAt: 9.
        start := next > start ifTrue: [next] ifFalse: [start + 1]].
    ^matches!
replace: subject size: size with: replacement
    | capacity destination length result needed |
    capacity := size + 1 max: 1.
    length := ByteArray new: 8.
    [
        destination := ExternalAddress allocate: capacity.
        length unsignedLongLongAt: 1 put: capacity.
        result := self class substitute: code subject: subject size: size data: data
            replacement: replacement length: replacement size output: destination capacity: length.
        result = -48
            ifTrue: [
                needed := length unsignedLongLongAt: 1.
                destination free.
                capacity := needed max: capacity * 2.
                true]
            ifFalse: [false]
    ] whileTrue.
    result < 0 ifTrue: [destination free. self error: 'PCRE2 substitute failed'].
    ^destination -> (length unsignedLongLongAt: 1)! !
'''
    rows.append(app('regexredux-smalltalk-pcre2',patterns+r'''
replace: pattern in: subject size: size with: replacement
    | regex answer |
    regex := BenchmarkPCRE pattern: pattern.
    [answer := regex replace: subject size: size with: replacement]
        ensure: [regex close].
    subject free.
    ^answer!
do: n
    | raw rawSize subject size cleaned regex count out changed |
    raw := self readInput.
    rawSize := raw size.
    subject := ExternalAddress allocate: (rawSize max: 1).
    LibC uniqueInstance memCopy: raw to: subject size: rawSize.
    cleaned := self replace: '>[^\n]*\n|\n' in: subject size: rawSize with: ''.
    subject := cleaned key.
    size := cleaned value.
    out := Stdio stdout.
    self patterns do: [:pattern |
        regex := BenchmarkPCRE pattern: pattern.
        [count := regex count: subject size: size] ensure: [regex close].
        out nextPutAll: pattern;
            nextPut: Character space;
            print: count;
            lf].
    self substitutions do: [:substitution |
        changed := self replace: substitution first in: subject size: cleaned value with: substitution second.
        subject := changed key.
        cleaned := changed].
    subject free.
    out lf;
        print: rawSize;
        lf;
        print: size;
        lf;
        print: cleaned value;
        lf
''',['Use standard PCRE2 regular expression compilation, JIT, nonoverlapping matching and ordered global substitution through counted general UFFI bindings. Match/substitution orchestrations and complete error/overflow/memory handling are Smalltalk application source; no C benchmark executor or literal DNA recognizer. Input memory copied once through standard LibC memcpy to stable native storage.'],extra=pcre))

    rows.append(app('revcomp-smalltalk-streaming',r'''
emit: header sequence: sequence to: out using: complement
    | bytes left right temp length |
    header ifNil: [^self].
    bytes := sequence asByteArray.
    left := 1.
    right := bytes size.
    [left <= right] whileTrue: [
        temp := complement at: (bytes at: left) + 1.
        bytes at: left put: (complement at: (bytes at: right) + 1).
        bytes at: right put: temp.
        left := left + 1.
        right := right - 1].
    out nextPutAll: header asByteArray;
        nextPut: 10.
    1 to: bytes size by: 60 do: [:i |
        length := 60 min: bytes size - i + 1.
        out next: length putAll: bytes startingAt: i;
            nextPut: 10]!
do: n
    | input out buffer header line complement |
    input := ZnCharacterReadStream on: (ZnBufferedReadStream on: Stdio stdin).
    out := ZnBufferedWriteStream on: Stdio stdout binary.
    buffer := WriteStream on: (String new: 1024).
    header := nil.
    complement := ByteArray new: 256 withAll: 0.
    'ABCDGHKMNRSTVWY' with: 'TVGHCDMKNYSABWR' do: [:a :b |
        complement at: a asInteger + 1 put: b asInteger.
        complement at: a asLowercase asInteger + 1 put: b asInteger].
    [input atEnd] whileFalse: [
        line := input nextLine.
        (line beginsWith: '>')
            ifTrue: [
                self emit: header sequence: buffer contents to: out using: complement.
                header := line.
                buffer := WriteStream on: (String new: 1024)]
            ifFalse: [(line beginsWith: ';') ifFalse: [buffer nextPutAll: line]]].
    self emit: header sequence: buffer contents to: out using: complement.
    out flush
''',['Handle arbitrary FASTA records rather than hard-code ONE/TWO/THREE. General byte complement lookup, in-place symmetric reversal, and standard buffered bulk output retain complete headers, both cases and every sequence byte.']))

    (D/'support/smalltalk/custom-candidates.json').write_text(json.dumps(rows,indent=2)+'\n')


if __name__=='__main__':
    main()
