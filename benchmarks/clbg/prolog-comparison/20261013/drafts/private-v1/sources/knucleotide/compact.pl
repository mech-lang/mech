:- module(clbg_knucleotide, [main/0]).
:- use_module('../../support/clbg_io').
:- use_module('../../support/clbg_records').
:- use_module(library(hashtable), [ht_new/1, ht_get/3, ht_put/3, ht_pairs/2]).
:- initialization(clbg_io:run(clbg_knucleotide:main), main).

find_third(Sequence) :-
    line(Line),
    (   Line == end_of_file
    ->  Sequence = ""
    ;   third_header(Line)
    ->  read_third([], Sequence)
    ;   find_third(Sequence)
    ).

read_third(Chunks, Sequence) :-
    line(Line),
    (   Line == end_of_file
    ->  sequence(Chunks, Sequence)
    ;   header(Line)
    ->  sequence(Chunks, Sequence)
    ;   comment(Line)
    ->  read_third(Chunks, Sequence)
    ;   read_third([Line|Chunks], Sequence)
    ).

base(Code, Bits) :-
    (   (Code =:= 65 ; Code =:= 97)
    ->  Bits = 0
    ;   (Code =:= 67 ; Code =:= 99)
    ->  Bits = 1
    ;   (Code =:= 71 ; Code =:= 103)
    ->  Bits = 2
    ;   (Code =:= 84 ; Code =:= 116)
    ->  Bits = 3
    ;   throw(error(domain_error(acgt_nucleotide, Code), base/2))
    ).

windows(Index, Length, Width, Mask, Sequence, Key0, Table) :-
    (   Index =< Length
    ->  get_string_code(Index, Sequence, Code),
        base(Code, Bits),
        Key is ((Key0 << 2) + Bits) /\ Mask,
        (   Index >= Width
        ->  (ht_get(Table, Key, Old) -> true ; Old = 0),
            Count is Old + 1,
            ht_put(Table, Key, Count)
        ;   true
        ),
        Next is Index + 1,
        windows(Next, Length, Width, Mask, Sequence, Key, Table)
    ;   true
    ).

all_maps([], _, _, []).
all_maps([Width|Widths], Sequence, Length, [Table|Tables]) :-
    ht_new(Table),
    Mask is (1 << (2 * Width)) - 1,
    windows(1, Length, Width, Mask, Sequence, 0, Table),
    all_maps(Widths, Sequence, Length, Tables).

rank_pairs([], []).
rank_pairs([Key-Count|Pairs], [Negative-Key|Ranked]) :-
    Negative is -Count,
    rank_pairs(Pairs, Ranked).

decode(0, _, []) :- !.
decode(Width, Key, [Code|Codes]) :-
    Shift is 2 * (Width - 1),
    Bits is (Key >> Shift) /\ 3,
    AlphabetIndex is Bits + 1,
    get_string_code(AlphabetIndex, "ACGT", Code),
    Remaining is Width - 1,
    decode(Remaining, Key, Codes).

emit_frequencies([], _, _).
emit_frequencies([Negative-Key|Rows], Width, Total) :-
    Count is -Negative,
    Percentage is 100.0 * Count / Total,
    decode(Width, Key, Word),
    format('~s ~3f~n', [Word, Percentage]),
    emit_frequencies(Rows, Width, Total).

frequencies(Table, Width, Length) :-
    ht_pairs(Table, Pairs),
    rank_pairs(Pairs, Ranked),
    keysort(Ranked, Sorted),
    Total is max(0, Length - Width + 1),
    (   Total > 0
    ->  emit_frequencies(Sorted, Width, Total)
    ;   true
    ),
    nl.

encode(Index, Length, Word, Key0, Key) :-
    (   Index =< Length
    ->  get_string_code(Index, Word, Code),
        base(Code, Bits),
        Key1 is (Key0 << 2) + Bits,
        Next is Index + 1,
        encode(Next, Length, Word, Key1, Key)
    ;   Key = Key0
    ).

emit_queries([], []).
emit_queries([Word|Words], [Table|Tables]) :-
    string_length(Word, Length),
    encode(1, Length, Word, 0, Key),
    (ht_get(Table, Key, Count) -> true ; Count = 0),
    format('~d\t~s~n', [Count, Word]),
    emit_queries(Words, Tables).

main :-
    find_third(Sequence),
    string_length(Sequence, Length),
    all_maps([1,2,3,4,6,12,18], Sequence, Length, Tables),
    Tables = [Singles,Doubles|QueryTables],
    frequencies(Singles, 1, Length),
    frequencies(Doubles, 2, Length),
    emit_queries(["GGT","GGTA","GGTATT","GGTATTTTAATT","GGTATTTTAATTTATAGT"],
                 QueryTables).
