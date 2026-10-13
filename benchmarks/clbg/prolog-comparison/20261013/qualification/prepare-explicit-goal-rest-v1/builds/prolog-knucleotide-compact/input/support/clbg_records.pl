:- module(clbg_records, [line/1, header/1, comment/1, third_header/1, sequence/2]).
line(Line) :-
    read_string(user_input, "\r\n", "", Separator, Raw),
    (   Separator =:= -1, Raw == ""
    ->  Line = end_of_file
    ;   Line = Raw
    ).

header(Line) :- sub_string(Line, 0, 1, _, ">").
comment(Line) :- sub_string(Line, 0, 1, _, ";").

third_header(Line) :-
    sub_string(Line, 0, 6, _, ">THREE"),
    string_length(Line, Length),
    (   Length =:= 6
    ->  true
    ;   get_string_code(7, Line, Next),
        (Next =:= 32 -> true ; Next =:= 9)
    ).

sequence(ReversedChunks, Sequence) :-
    reverse(ReversedChunks, Chunks),
    atomics_to_string(Chunks, "", Sequence).
