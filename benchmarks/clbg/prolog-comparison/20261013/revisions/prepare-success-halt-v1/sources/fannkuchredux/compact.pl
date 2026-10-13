:- module(clbg_fannkuchredux, [main/0]).
:- use_module('../../support/clbg_io').
:- initialization(clbg_io:run(clbg_fannkuchredux:main), main).

initial(At, N, Permutation, Rotations) :-
    (   At =< N
    ->  Value is At - 1,
        nb_setarg(At, Permutation, Value),
        nb_setarg(At, Rotations, 0),
        Next is At + 1,
        initial(Next, N, Permutation, Rotations)
    ;   true
    ).

factorial(0, 1) :- !.
factorial(N, Total) :-
    Previous is N - 1,
    factorial(Previous, Prefix),
    Total is N * Prefix.

copy_permutation(At, N, Original, Copy) :-
    (   At =< N
    ->  arg(At, Original, Value),
        nb_setarg(At, Copy, Value),
        Next is At + 1,
        copy_permutation(Next, N, Original, Copy)
    ;   true
    ).

reverse_prefix(Left, Right, Permutation) :-
    (   Left < Right
    ->  arg(Left, Permutation, A),
        arg(Right, Permutation, B),
        nb_setarg(Left, Permutation, B),
        nb_setarg(Right, Permutation, A),
        NextLeft is Left + 1,
        NextRight is Right - 1,
        reverse_prefix(NextLeft, NextRight, Permutation)
    ;   true
    ).

flips(Permutation, Accumulator, Count) :-
    arg(1, Permutation, First),
    (   First =:= 0
    ->  Count = Accumulator
    ;   Width is First + 1,
        reverse_prefix(1, Width, Permutation),
        Next is Accumulator + 1,
        flips(Permutation, Next, Count)
    ).

shift_prefix(At, Width, Permutation) :-
    (   At < Width
    ->  From is At + 1,
        arg(From, Permutation, Value),
        nb_setarg(At, Permutation, Value),
        shift_prefix(From, Width, Permutation)
    ;   true
    ).

rotate_prefix(Width, Permutation) :-
    arg(1, Permutation, First),
    shift_prefix(1, Width, Permutation),
    nb_setarg(Width, Permutation, First).

next_permutation(Width, N, Permutation, Rotations) :-
    (   Width =< N
    ->  rotate_prefix(Width, Permutation),
        arg(Width, Rotations, Previous),
        NextCount is Previous + 1,
        (   NextCount < Width
        ->  nb_setarg(Width, Rotations, NextCount)
        ;   nb_setarg(Width, Rotations, 0),
            NextWidth is Width + 1,
            next_permutation(NextWidth, N, Permutation, Rotations)
        )
    ;   throw(error(unexpected_permutation_carry, next_permutation/4))
    ).

permutations(Rank, Total, N, Permutation, Rotations, Checksum0, Maximum0,
             Checksum, Maximum) :-
    functor(Work, permutation, N),
    copy_permutation(1, N, Permutation, Work),
    flips(Work, 0, Count),
    (   Rank mod 2 =:= 0
    ->  Checksum1 is Checksum0 + Count
    ;   Checksum1 is Checksum0 - Count
    ),
    Maximum1 is max(Maximum0, Count),
    NextRank is Rank + 1,
    (   NextRank < Total
    ->  next_permutation(2, N, Permutation, Rotations),
        permutations(NextRank, Total, N, Permutation, Rotations,
                     Checksum1, Maximum1, Checksum, Maximum)
    ;   Checksum = Checksum1,
        Maximum = Maximum1
    ).

main :-
    size(N),
    (   N >= 1, N =< 12
    ->  true
    ;   throw(error(domain_error(fannkuch_size_1_to_12, N), main/0))
    ),
    functor(Permutation, permutation, N),
    functor(Rotations, rotations, N),
    initial(1, N, Permutation, Rotations),
    factorial(N, Total),
    permutations(0, Total, N, Permutation, Rotations, 0, 0, Checksum, Maximum),
    format('~d~nPfannkuchen(~d) = ~d~n', [Checksum, N, Maximum]).
