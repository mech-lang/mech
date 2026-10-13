:- module(clbg_prepare, [main/0]).
:- use_module(library(http/json), [json_write_dict/2]).
:- use_module(library(shlib), [current_foreign_library/2]).
:- use_module(library(vm), [vm_list/1]).
:- initialization(main, main).

term_text(Term, Text) :- with_output_to(string(Text), write_canonical(Term)).

version_components([Major,Minor,Patch]) :-
    current_prolog_flag(version_data, swi(Major,Minor,Patch,_)).

runtime(Runtime) :-
    version_components(Components),
    current_prolog_flag(version_data, Version),
    term_text(Version, VersionText),
    current_prolog_flag(arch, Architecture),
    current_prolog_flag(home, Home),
    current_prolog_flag(bounded, Bounded),
    current_prolog_flag(gc, GC),
    current_prolog_flag(stack_limit, StackLimit),
    Runtime = _{version_components:Components, version_data:VersionText,
                architecture:Architecture, home:Home, bounded:Bounded,
                gc:GC, stack_limit:StackLimit}.

loaded_sources(Sources) :-
    findall(File, source_file(File), All),
    sort(All, Sources).

foreign_libraries(Libraries) :-
    findall(_{file:FileText,public:PublicText,resolved_path:Path},
            (current_foreign_library(File, Public),
             term_text(File, FileText), term_text(Public, PublicText),
             absolute_file_name(File, Path, [file_type(executable),access(read),file_errors(error)])),
            Libraries).

written_predicate(Module, Name, Arity, Source) :-
    current_module(Module),
    current_predicate(Module:Name/Arity),
    functor(Head, Name, Arity),
    predicate_property(Module:Head, file(Source)),
    \+ predicate_property(Module:Head, imported_from(_)).

within(Root, File) :-
    atom_concat(Root, '/', Prefix),
    sub_atom(File, 0, _, _, Prefix).

write_wam(Stream, Sources, Root) :-
    forall((member(Source, Sources), within(Root, Source),
            written_predicate(Module, Name, Arity, Source)),
           (format(Stream, '~nsource ~q~n', [Source]),
            with_output_to(Stream, vm_list(Module:Name/Arity)))).

prepare(compile, Target, _, _) :- qcompile(Target).
prepare(inspect, Target, _, _) :- load_files(Target, [silent(true)]).
prepare(runtime, _, _, _).

checked :-
    current_prolog_flag(argv, [Mode,Target,Receipt,Wam,AuthoredRoot]),
    memberchk(Mode, [compile,inspect,runtime]),
    prepare(Mode, Target, Receipt, Wam),
    runtime(Runtime),
    loaded_sources(Sources),
    foreign_libraries(Foreign),
    setup_call_cleanup(open(Wam, write, Stream, [encoding(utf8)]),
                       write_wam(Stream, Sources, AuthoredRoot), close(Stream)),
    setup_call_cleanup(open(Receipt, write, Out, [encoding(utf8)]),
                       json_write_dict(Out, _{mode:Mode,target:Target,
                           runtime:Runtime,loaded_sources:Sources,
                           foreign_libraries:Foreign,wam:Wam}), close(Out)).

main :-
    catch(checked, Error, (print_message(error, Error), halt(1))),
    halt(0).
