:- module(prolog_source_scan_v1, [main/0]).
:- use_module(library(http/json), [json_write_dict/2]).

% Unexecuted proposal. This reader never consults/loads the input, applies its
% directives, expands terms, compiles QLF, or invokes any benchmark predicate.
% Launch only through a reviewed owner holding /tmp/mech-clbg-timing.lock.

text(Term, Text) :- term_string(Term, Text, [quoted(true), ignore_ops(true)]).
char_position(Stream, Count) :-
    stream_property(Stream, position(Position)),
    stream_position_data(char_count, Position, Count).

operators(Operators) :-
    findall(operator{priority:Priority, type:TypeText, name:NameText},
            (current_op(Priority, Type, Name), atom_string(Type, TypeText),
             atom_string(Name, NameText)), Unsorted),
    sort(Unsorted, Operators).

variable_identity(Variable, Variables, Names, Index, NameText) :-
    nth0(Index, Variables, Candidate), Variable == Candidate, !,
    variable_label(Variable, Names, NameText).
variable_label(Variable, [Name=Candidate|_], Text) :-
    Variable == Candidate, !, atom_string(Name, Text).
variable_label(Variable, [_|Rest], Text) :- !,
    variable_label(Variable, Rest, Text).
variable_label(_, [], "_").

node(Term, parentheses_term_position(Start, End, Content), Variables, Names,
     _{kind:"group", start:Start, end:End, children:[Child]}) :- !,
    node(Term, Content, Variables, Names, Child).
node(Term, list_position(Start, End, Positions, TailPosition), Variables, Names,
     _{kind:"list", start:Start, end:End, children:Children, tail:Tail}) :- !,
    list_nodes(Term, Positions, Variables, Names, Children, Rest),
    ( TailPosition == none -> Tail = null
    ; node(Rest, TailPosition, Variables, Names, Tail)
    ).
node(Term, string_position(Start, End), _, _,
     _{kind:"string", start:Start, end:End, value:Term}) :- !,
    string(Term).
node(Term, term_position(Start, End, FunctorStart, FunctorEnd, Positions),
     Variables, Names,
     _{kind:"term", start:Start, end:End, name:NameText,
       functor_start:FunctorStart, functor_end:FunctorEnd, children:Children}) :- !,
    compound_name_arguments(Term, Name, Arguments), atom_string(Name, NameText),
    same_length(Arguments, Positions),
    child_nodes(Arguments, Positions, Variables, Names, Children).
node(Term, Start-End, Variables, Names, Node) :- !,
    ( var(Term)
    -> variable_identity(Term, Variables, Names, Index, Name),
       Node = _{kind:"variable", start:Start, end:End, id:Index, name:Name}
    ; atom(Term)
    -> atom_string(Term, Value),
       Node = _{kind:"atom", start:Start, end:End, value:Value}
    ; number(Term)
    -> text(Term, Value),
       Node = _{kind:"number", start:Start, end:End, value:Value}
    ; text(Term, Value),
      Node = _{kind:"unsupported", start:Start, end:End, detail:Value}
    ).
node(_, Position, _, _, _{kind:"unsupported-position", detail:Detail}) :-
    text(Position, Detail).

child_nodes([], [], _, _, []).
child_nodes([Term|Terms], [Position|Positions], Variables, Names, [Node|Nodes]) :-
    node(Term, Position, Variables, Names, Node),
    child_nodes(Terms, Positions, Variables, Names, Nodes).
list_nodes(Rest, [], _, _, [], Rest).
list_nodes([Term|Terms], [Position|Positions], Variables, Names, [Node|Nodes], Rest) :-
    node(Term, Position, Variables, Names, Node),
    list_nodes(Terms, Positions, Variables, Names, Nodes, Rest).

comments([], []).
comments([Position-Text|Rest], [Comment|Comments]) :-
    stream_position_data(char_count, Position, Start),
    string_length(Text, Length), End is Start + Length,
    Comment = _{start:Start, end:End, text:Text}, comments(Rest, Comments).

terms(Stream, Forms, Comments, End) :-
    char_position(Stream, Before),
    read_term(Stream, Term, [module(prolog_source_scan_v1),
        character_escapes(true), double_quotes(string), backquoted_string(false),
        syntax_errors(error), cycles(false), dotlists(false),
        variable_names(Names), variables(Variables), subterm_positions(Position),
        comments(RawComments), quasi_quotations(Quotations)]),
    char_position(Stream, After), comments(RawComments, TheseComments),
    ( Term == end_of_file
    -> Forms = [], Comments = TheseComments, End = After
    ; ( Quotations == [] -> QuotationCount = 0
      ; length(Quotations, QuotationCount)
      ),
      node(Term, Position, Variables, Names, Node),
      Forms = [_{read_start:Before, read_end:After,
                 quasi_quotation_count:QuotationCount, node:Node}|Rest],
      terms(Stream, Rest, LaterComments, End),
      append(TheseComments, LaterComments, Comments)
    ).

checked :-
    current_prolog_flag(argv, [Source]),
    setup_call_cleanup(open(Source, read, Input, [encoding(utf8), bom(false)]),
                       read_string(Input, _, SourceText), close(Input)),
    operators(Before),
    setup_call_cleanup(open_string(SourceText, Stream),
                       terms(Stream, Forms, Comments, End), close(Stream)),
    operators(After),
    current_prolog_flag(version_data, swi(Major, Minor, Patch, _)),
    json_write_dict(current_output, _{schema:"prolog-source-reader-v1",
        source:Source, source_text:SourceText, version:[Major,Minor,Patch],
        reader_settings:_{module:"prolog_source_scan_v1", character_escapes:true,
            double_quotes:"string", backquoted_string:false, cycles:false,
            dotlists:false, quasi_quotations:"collected-not-evaluated"},
        operators_before:Before, operators_after:After,
        forms:Forms, comments:Comments, eof_char_count:End}),
    nl.
main :- catch(checked, Error, (print_message(error, Error), halt(1))), halt(0).
