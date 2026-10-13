:- module(clbg_halt_diagnostic, [direct/0, wrapped/0]).

% Diagnostic-only controls. The frozen feature probe is loaded by the command,
% but its feature/compiler main is never called. Each command receives a fresh
% event file as its sole argument. The reviewed owner must require nonexistence.
event(Term) :-
    current_prolog_flag(argv, [Events]),
    setup_call_cleanup(open(Events, append, Out, [encoding(utf8)]),
        (write_term(Out, Term, [quoted(true),ignore_ops(true)]),
         write(Out, '.'), nl(Out), flush_output(Out)), close(Out)).

observed_halt :-
    setup_call_catcher_cleanup(true,
        (event(before_halt(0)), halt(0)),
        Reason, event(cleanup(Reason))).

direct :-
    event(started(direct)),
    observed_halt,
    event(unexpected_return(direct)),
    halt(3).

wrapped :-
    event(started(wrapped)),
    catch(observed_halt, Error,
        (event(recovery(Error)),
         print_message(error, Error),
         event(before_recovery_halt(1)),
         halt(1))),
    event(unexpected_return(wrapped)),
    halt(3).
