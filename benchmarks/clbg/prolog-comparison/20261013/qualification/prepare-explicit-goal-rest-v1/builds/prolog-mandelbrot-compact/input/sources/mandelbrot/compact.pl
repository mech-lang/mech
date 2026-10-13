:- module(clbg_mandelbrot, [main/0]).
:- use_module('../../support/clbg_io').
:- initialization(clbg_io:run(clbg_mandelbrot:main), main).

escape(0, _, _, _, _, 1) :- !.
escape(Iterations, CR, CI, ZR, ZI, Bit) :-
    ZR2 is ZR * ZR,
    ZI2 is ZI * ZI,
    NextZI is 2.0 * ZR * ZI + CI,
    NextZR is ZR2 - ZI2 + CR,
    Magnitude is NextZR * NextZR + NextZI * NextZI,
    (   Magnitude > 4.0
    ->  Bit = 0
    ;   Remaining is Iterations - 1,
        escape(Remaining, CR, CI, NextZR, NextZI, Bit)
    ).

pixel(X, N, _, 0) :- X >= N, !.
pixel(X, N, CI, Bit) :-
    CR is 2.0 * X / N - 1.5,
    escape(50, CR, CI, 0.0, 0.0, Bit).

packed(8, _, _, _, Byte, Byte) :- !.
packed(Bits, X, N, CI, Accumulator, Byte) :-
    pixel(X, N, CI, Bit),
    NextAccumulator is Accumulator * 2 + Bit,
    NextBits is Bits + 1,
    NextX is X + 1,
    packed(NextBits, NextX, N, CI, NextAccumulator, Byte).

row(X, N, _) :- X >= N, !.
row(X, N, CI) :-
    packed(0, X, N, CI, 0, Byte),
    put_code(Byte),
    NextX is X + 8,
    row(NextX, N, CI).

rows(Y, N) :- Y >= N, !.
rows(Y, N) :-
    CI is 2.0 * Y / N - 1.0,
    row(0, N, CI),
    NextY is Y + 1,
    rows(NextY, N).

main :-
    size(N),
    format('P4~n~d ~d~n', [N, N]),
    rows(0, N).
