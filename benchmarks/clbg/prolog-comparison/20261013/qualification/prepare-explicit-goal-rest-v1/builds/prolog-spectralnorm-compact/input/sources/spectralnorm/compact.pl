:- module(clbg_spectralnorm, [main/0]).
:- use_module('../../support/clbg_io').
:- initialization(clbg_io:run(clbg_spectralnorm:main), main).

coefficient(I, J, Value) :-
    Sum is I + J,
    Denominator is (Sum * (Sum + 1)) div 2 + I + 1,
    Value is 1.0 / Denominator.

fill(Index, N, Vector) :-
    (   Index =< N
    ->  arg(Index, Vector, 1.0),
        Next is Index + 1,
        fill(Next, N, Vector)
    ;   true
    ).

row(J, N, _, _, _, Accumulator, Accumulator) :- J >= N, !.
row(J, N, I, Transpose, Input, Accumulator, Sum) :-
    (   Transpose == true
    ->  coefficient(J, I, A)
    ;   coefficient(I, J, A)
    ),
    Position is J + 1,
    arg(Position, Input, Value),
    Term is A * Value,
    NextAccumulator is Accumulator + Term,
    NextJ is J + 1,
    row(NextJ, N, I, Transpose, Input, NextAccumulator, Sum).

rows(I, N, _, _, _) :- I >= N, !.
rows(I, N, Transpose, Input, Output) :-
    row(0, N, I, Transpose, Input, 0.0, Value),
    Position is I + 1,
    arg(Position, Output, Value),
    NextI is I + 1,
    rows(NextI, N, Transpose, Input, Output).

apply(Input, N, Output) :-
    functor(Output, vector, N),
    rows(0, N, false, Input, Output).

apply_transpose(Input, N, Output) :-
    functor(Output, vector, N),
    rows(0, N, true, Input, Output).

apply_ata(Input, N, Output) :-
    apply(Input, N, Intermediate),
    apply_transpose(Intermediate, N, Output).

power(0, U, V, _, U, V) :- !.
power(Iterations, Input, _, N, U, V) :-
    apply_ata(Input, N, NextV),
    apply_ata(NextV, N, NextU),
    Remaining is Iterations - 1,
    power(Remaining, NextU, NextV, N, U, V).

dots(Index, N, _, _, UV, VV, UV, VV) :- Index > N, !.
dots(Index, N, U, V, UV0, VV0, UV, VV) :-
    arg(Index, U, UI),
    arg(Index, V, VI),
    UV1 is UV0 + UI * VI,
    VV1 is VV0 + VI * VI,
    Next is Index + 1,
    dots(Next, N, U, V, UV1, VV1, UV, VV).

main :-
    size(N),
    (   N =:= 0
    ->  format('0.000000000~n', [])
    ;   functor(Seed, vector, N),
        fill(1, N, Seed),
        power(10, Seed, Seed, N, U, V),
        dots(1, N, U, V, 0.0, 0.0, UV, VV),
        Norm is sqrt(UV / VV),
        format('~9f~n', [Norm])
    ).
