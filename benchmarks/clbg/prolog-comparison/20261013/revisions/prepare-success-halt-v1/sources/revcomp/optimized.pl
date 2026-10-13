:- module(clbg_revcomp, [main/0]).
:- use_module('../../support/clbg_io').
:- use_module('../../support/clbg_records').
:- initialization(clbg_io:run(clbg_revcomp:main), main).

complement(Code, Complement) :-
    Symbols = "ACBDGHKMNSRUTWVYacbdghkmnsrutwvy",
    Complements = "TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR",
    find_symbol(1, Symbols, Complements, Code, Complement).

find_symbol(Index, Symbols, Complements, Code, Complement) :-
    (   Index =< 32
    ->  get_string_code(Index, Symbols, At),
        (   At =:= Code
        ->  get_string_code(Index, Complements, Complement)
        ;   Next is Index + 1,
            find_symbol(Next, Symbols, Complements, Code, Complement)
        )
    ;   throw(error(domain_error(iupac_nucleotide, Code), complement/2))
    ).

reverse_codes(0, Position, _, [], Position) :- !.
reverse_codes(Count, Position0, Sequence, [Code|Rest], Position) :-
    get_string_code(Position0, Sequence, Original),
    complement(Original, Code),
    Position1 is Position0 - 1,
    Remaining is Count - 1,
    reverse_codes(Remaining, Position1, Sequence, Rest, Position).

emit_sequence(0, _) :- !.
emit_sequence(Position0, Sequence) :-
    Width is min(60, Position0),
    reverse_codes(Width, Position0, Sequence, Codes, Position1),
    format('~s~n', [Codes]),
    emit_sequence(Position1, Sequence).

emit_record(none, _) :- !.
emit_record(Header, Chunks) :-
    format('~s~n', [Header]),
    sequence(Chunks, Sequence),
    string_length(Sequence, Length),
    emit_sequence(Length, Sequence).

records(Header, Chunks) :-
    line(Line),
    (   Line == end_of_file
    ->  emit_record(Header, Chunks)
    ;   header(Line)
    ->  emit_record(Header, Chunks),
        records(Line, [])
    ;   comment(Line)
    ->  records(Header, Chunks)
    ;   Header == none
    ->  records(Header, Chunks)
    ;   records(Header, [Line|Chunks])
    ).

main :- records(none, []).
