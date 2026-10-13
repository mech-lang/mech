:- module(clbg_nbody, [main/0]).
:- use_module('../../support/clbg_io').
:- initialization(clbg_io:run(clbg_nbody:main), main).

new_body(X, Y, Z, VX, VY, VZ, Mass, Body) :-
    functor(Body, body, 7),
    arg(1, Body, X),
    arg(2, Body, Y),
    arg(3, Body, Z),
    arg(4, Body, VX),
    arg(5, Body, VY),
    arg(6, Body, VZ),
    arg(7, Body, Mass).

new_planet(X, Y, Z, VX0, VY0, VZ0, Mass0, Solar, Days, Body) :-
    VX is VX0 * Days,
    VY is VY0 * Days,
    VZ is VZ0 * Days,
    Mass is Mass0 * Solar,
    new_body(X, Y, Z, VX, VY, VZ, Mass, Body).

initial_bodies(Bodies, Solar) :-
    Solar is 4.0 * 3.141592653589793 * 3.141592653589793,
    Days = 365.24,
    functor(Bodies, bodies, 5),
    arg(1, Bodies, Sun),
    new_body(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, Solar, Sun),
    arg(2, Bodies, Jupiter),
    new_planet(4.84143144246472090, -1.16032004402742839,
               -1.03622044471123109e-1, 1.66007664274403694e-3,
               7.69901118419740425e-3, -6.90460016972063023e-5,
               9.54791938424326609e-4, Solar, Days, Jupiter),
    arg(3, Bodies, Saturn),
    new_planet(8.34336671824457987, 4.12479856412430479,
               -4.03523417114321381e-1, -2.76742510726862411e-3,
               4.99852801234917238e-3, 2.30417297573763929e-5,
               2.85885980666130812e-4, Solar, Days, Saturn),
    arg(4, Bodies, Uranus),
    new_planet(1.28943695621391310e1, -1.51111514016986312e1,
               -2.23307578892655734e-1, 2.96460137564761618e-3,
               2.37847173959480950e-3, -2.96589568540237556e-5,
               4.36624404335156298e-5, Solar, Days, Uranus),
    arg(5, Bodies, Neptune),
    new_planet(1.53796971148509165e1, -2.59193146099879641e1,
               1.79258772950371181e-1, 2.68067772490389322e-3,
               1.62824170038242295e-3, -9.51592254519715870e-5,
               5.15138902046611451e-5, Solar, Days, Neptune).

momentum(Index, Bodies, PX0, PY0, PZ0, PX, PY, PZ) :-
    (   Index =< 5
    ->  arg(Index, Bodies, Body),
        arg(4, Body, VX),
        arg(5, Body, VY),
        arg(6, Body, VZ),
        arg(7, Body, Mass),
        PX1 is PX0 + Mass * VX,
        PY1 is PY0 + Mass * VY,
        PZ1 is PZ0 + Mass * VZ,
        Next is Index + 1,
        momentum(Next, Bodies, PX1, PY1, PZ1, PX, PY, PZ)
    ;   PX = PX0, PY = PY0, PZ = PZ0
    ).

offset_momentum(Bodies, Solar) :-
    momentum(1, Bodies, 0.0, 0.0, 0.0, PX, PY, PZ),
    arg(1, Bodies, Sun),
    VX is -PX / Solar,
    VY is -PY / Solar,
    VZ is -PZ / Solar,
    nb_setarg(4, Sun, VX),
    nb_setarg(5, Sun, VY),
    nb_setarg(6, Sun, VZ).

distance(Left, Right, DX, DY, DZ, Squared) :-
    arg(1, Left, LX), arg(1, Right, RX),
    arg(2, Left, LY), arg(2, Right, RY),
    arg(3, Left, LZ), arg(3, Right, RZ),
    DX is LX - RX,
    DY is LY - RY,
    DZ is LZ - RZ,
    Squared is DX * DX + DY * DY + DZ * DZ.

velocity(Slot, Delta, Magnitude, LeftMass, RightMass, Left, Right) :-
    arg(Slot, Left, LV0),
    arg(Slot, Right, RV0),
    LV is LV0 - Delta * RightMass * Magnitude,
    RV is RV0 + Delta * LeftMass * Magnitude,
    nb_setarg(Slot, Left, LV),
    nb_setarg(Slot, Right, RV).

force(Left, Right, Step) :-
    distance(Left, Right, DX, DY, DZ, Squared),
    Radius is sqrt(Squared),
    Magnitude is Step / (Squared * Radius),
    arg(7, Left, LeftMass),
    arg(7, Right, RightMass),
    velocity(4, DX, Magnitude, LeftMass, RightMass, Left, Right),
    velocity(5, DY, Magnitude, LeftMass, RightMass, Left, Right),
    velocity(6, DZ, Magnitude, LeftMass, RightMass, Left, Right).

right_forces(Index, Left, Bodies, Step) :-
    (   Index =< 5
    ->  arg(Index, Bodies, Right),
        force(Left, Right, Step),
        Next is Index + 1,
        right_forces(Next, Left, Bodies, Step)
    ;   true
    ).

forces(Index, Bodies, Step) :-
    (   Index < 5
    ->  arg(Index, Bodies, Left),
        Next is Index + 1,
        right_forces(Next, Left, Bodies, Step),
        forces(Next, Bodies, Step)
    ;   true
    ).

positions(Index, Bodies, Step) :-
    (   Index =< 5
    ->  arg(Index, Bodies, Body),
        arg(1, Body, X0), arg(4, Body, VX),
        arg(2, Body, Y0), arg(5, Body, VY),
        arg(3, Body, Z0), arg(6, Body, VZ),
        X is X0 + Step * VX,
        Y is Y0 + Step * VY,
        Z is Z0 + Step * VZ,
        nb_setarg(1, Body, X),
        nb_setarg(2, Body, Y),
        nb_setarg(3, Body, Z),
        Next is Index + 1,
        positions(Next, Bodies, Step)
    ;   true
    ).

advance(0, _, _) :- !.
advance(Count, Bodies, Step) :-
    forces(1, Bodies, Step),
    positions(1, Bodies, Step),
    Remaining is Count - 1,
    advance(Remaining, Bodies, Step).

potential(Index, Left, Bodies, Energy0, Energy) :-
    (   Index =< 5
    ->  arg(Index, Bodies, Right),
        distance(Left, Right, _, _, _, Squared),
        Radius is sqrt(Squared),
        arg(7, Left, LeftMass),
        arg(7, Right, RightMass),
        Energy1 is Energy0 - (LeftMass * RightMass) / Radius,
        Next is Index + 1,
        potential(Next, Left, Bodies, Energy1, Energy)
    ;   Energy = Energy0
    ).

energy(Index, Bodies, Energy0, Energy) :-
    (   Index =< 5
    ->  arg(Index, Bodies, Body),
        arg(4, Body, VX),
        arg(5, Body, VY),
        arg(6, Body, VZ),
        arg(7, Body, Mass),
        Kinetic is 0.5 * Mass * (VX * VX + VY * VY + VZ * VZ),
        Energy1 is Energy0 + Kinetic,
        Next is Index + 1,
        potential(Next, Body, Bodies, Energy1, Energy2),
        energy(Next, Bodies, Energy2, Energy)
    ;   Energy = Energy0
    ).

main :-
    size(N),
    initial_bodies(Bodies, Solar),
    offset_momentum(Bodies, Solar),
    energy(1, Bodies, 0.0, Before),
    format('~9f~n', [Before]),
    advance(N, Bodies, 0.01),
    energy(1, Bodies, 0.0, After),
    format('~9f~n', [After]).
