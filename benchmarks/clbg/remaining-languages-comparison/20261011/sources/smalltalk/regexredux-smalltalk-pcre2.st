
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
"Computer Language Benchmarks Game. Local native Pharo adaptations, 2026."
Object subclass: #BenchmarksGame
    instanceVariableNames: ''
    classVariableNames: ''
    poolDictionaries: ''
    category: 'Benchmarks-Game'!
!BenchmarksGame class methodsFor: 'application'!

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
! !
