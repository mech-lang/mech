:- module(clbg_io, [run/1, size/1]).
:- meta_predicate run(0).

run(Goal) :-
    catch(run_checked(Goal), Error, report_error(Error)),
    halt(0).

run_checked(Goal) :-
    set_stream(user_input, encoding(octet)),
    set_stream(user_output, encoding(octet)),
    (   call(Goal)
    ->  flush_output(user_output)
    ;   throw(error(application_goal_failed, clbg_io:run/1))
    ).

report_error(Error) :-
    print_message(error, Error),
    halt(1).

size(N) :-
    current_prolog_flag(argv, Arguments),
    (   Arguments = [Text|_],
        catch(atom_number(Text, N), _, fail),
        integer(N),
        N >= 0
    ->  true
    ;   throw(error(domain_error(nonnegative_integer_argument, Arguments), clbg_io:size/1))
    ).
