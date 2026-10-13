:- module(clbg_fasta, [main/0]).
:- use_module('../../support/clbg_io').
:- initialization(clbg_io:run(clbg_fasta:main), main).

alu("GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA").

iub([97-0.27,99-0.12,103-0.12,116-0.27,
     66-0.02,68-0.02,72-0.02,75-0.02,77-0.02,78-0.02,
     82-0.02,83-0.02,86-0.02,87-0.02,89-0.02]).
homo([97-0.3029549426680,99-0.1979883004921,
      103-0.1975473066391,116-0.3015094502008]).

cumulative([], _, []).
cumulative([Code-Probability|Rest], Sum0, [Code-Sum|CDF]) :-
    Sum is Sum0 + Probability,
    cumulative(Rest, Sum, CDF).

repeat_codes(0, Position, _, _, [], Position) :- !.
repeat_codes(Count, Position0, Alu, Length, [Code|Rest], Position) :-
    Index is Position0 + 1,
    get_string_code(Index, Alu, Code),
    Position1 is (Position0 + 1) mod Length,
    Remaining is Count - 1,
    repeat_codes(Remaining, Position1, Alu, Length, Rest, Position).

repeat_lines(0, _, _, _) :- !.
repeat_lines(Count, Position0, Alu, Length) :-
    Width is min(60, Count),
    repeat_codes(Width, Position0, Alu, Length, Codes, Position1),
    format('~s~n', [Codes]),
    Remaining is Count - Width,
    repeat_lines(Remaining, Position1, Alu, Length).

select([Code-_], _, Code) :- !.
select([Code-Cutoff|Rest], Draw, Selected) :-
    (   Draw < Cutoff
    ->  Selected = Code
    ;   select(Rest, Draw, Selected)
    ).

random_codes(0, _, Seed, [], Seed) :- !.
random_codes(Count, CDF, Seed0, [Code|Rest], Seed) :-
    Seed1 is (Seed0 * 3877 + 29573) mod 139968,
    Draw is Seed1 / 139968.0,
    select(CDF, Draw, Code),
    Remaining is Count - 1,
    random_codes(Remaining, CDF, Seed1, Rest, Seed).

random_lines(0, _, Seed, Seed) :- !.
random_lines(Count, CDF, Seed0, Seed) :-
    Width is min(60, Count),
    random_codes(Width, CDF, Seed0, Codes, Seed1),
    format('~s~n', [Codes]),
    Remaining is Count - Width,
    random_lines(Remaining, CDF, Seed1, Seed).

main :-
    size(N),
    alu(Alu),
    string_length(Alu, AluLength),
    RepeatCount is 2 * N,
    format('>ONE Homo sapiens alu~n', []),
    repeat_lines(RepeatCount, 0, Alu, AluLength),
    iub(IUB),
    cumulative(IUB, 0.0, IUBCDF),
    IUBCount is 3 * N,
    format('>TWO IUB ambiguity codes~n', []),
    random_lines(IUBCount, IUBCDF, 42, NextSeed),
    homo(Homo),
    cumulative(Homo, 0.0, HomoCDF),
    HomoCount is 5 * N,
    format('>THREE Homo sapiens frequency~n', []),
    random_lines(HomoCount, HomoCDF, NextSeed, _).
