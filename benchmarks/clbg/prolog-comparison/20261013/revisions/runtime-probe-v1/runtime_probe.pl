:- module(clbg_runtime_probe, [main/0]).
:- use_module(prepare_without_main, []).
:- use_module(library(http/json), [json_write_dict/2]).
:- use_module(library(hashtable), [ht_new/1,ht_put/3,ht_get/3,ht_size/2]).
:- use_module(library(pcre), [re_compile/3,re_foldl/6,re_replace/5,re_config/1]).

increment(_, Before, After) :- After is Before + 1.

checked :-
    current_prolog_flag(argv, [Receipt]),
    clbg_prepare:runtime(Runtime),
    Large is (1 << 220) + 12345,
    Quotient is Large div 97,
    Remainder is Large mod 97,
    Floor is -7 div 3,
    Modulo is -7 mod 3,
    Root is sqrt(4.0),
    ht_new(Table),
    ht_put(Table, 11, 1), ht_put(Table, 23, 2),
    ht_put(Table, 37, 3), ht_put(Table, 41, 4),
    ht_put(Table, 11, 5),
    ht_get(Table, 11, Five),
    ht_size(Table, Size),
    arg(2, Table, Capacity),
    re_config(version(PCREVersion)),
    re_compile("a", Regex, [jit(false),capture_type(range)]),
    re_foldl(increment, Regex, "aba", 0, MatchCount, []),
    re_replace("a"/g, "x", "aba", Replaced, [jit(false)]),
    clbg_prepare:loaded_sources(Sources),
    clbg_prepare:foreign_libraries(Foreign),
    setup_call_cleanup(open(Receipt, write, Out, [encoding(utf8)]),
        json_write_dict(Out, _{runtime:Runtime,large:Large,quotient:Quotient,remainder:Remainder,
            negative_floor:Floor,negative_modulo:Modulo,square_root:Root,
            hash_value:Five,hash_size:Size,hash_capacity:Capacity,
            pcre_version:PCREVersion,regex_matches:MatchCount,regex_replacement:Replaced,
            regex_jit_requested:false,loaded_sources:Sources,foreign_libraries:Foreign}), close(Out)),
    halt(0).

main :- catch(checked, Error, (print_message(error, Error), halt(1))).
