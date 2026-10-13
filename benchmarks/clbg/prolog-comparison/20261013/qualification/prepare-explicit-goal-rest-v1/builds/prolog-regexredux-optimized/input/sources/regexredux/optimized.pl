:- module(clbg_regexredux, [main/0]).
:- use_module('../../support/clbg_io').
:- use_module(library(pcre), [re_foldl/6, re_replace/5]).
:- initialization(clbg_io:run(clbg_regexredux:main), main).

variants(["agggtaaa|tttaccct",
          "[cgt]gggtaaa|tttaccc[acg]",
          "a[act]ggtaaa|tttacc[agt]t",
          "ag[act]gtaaa|tttac[agt]ct",
          "agg[act]taaa|ttta[agt]cct",
          "aggg[acg]aaa|ttt[cgt]ccct",
          "agggt[cgt]aa|tt[acg]accct",
          "agggta[cgt]a|t[acg]taccct",
          "agggtaa[cgt]|[acg]ttaccct"]).

count_match(_, Before, After) :- After is Before + 1.

counts([], _).
counts([Pattern|Patterns], Sequence) :-
    re_foldl(count_match, Pattern, Sequence, 0, Count,
             [capture_type(range), jit(false)]),
    format('~s ~d~n', [Pattern, Count]),
    counts(Patterns, Sequence).

replacements([], Sequence, Sequence).
replacements([Pattern-With|Rest], Before, After) :-
    re_replace(Pattern/g, With, Before, Next, [jit(false)]),
    replacements(Rest, Next, After).

main :-
    read_string(user_input, _, Input),
    string_length(Input, OriginalLength),
    re_replace(">.*\n|\n"/g, "", Input, Sequence, [jit(false)]),
    string_length(Sequence, CleanLength),
    variants(Variants),
    counts(Variants, Sequence),
    replacements(["tHa[Nt]"-"<4>",
                  "aND|caN|Ha[DS]|WaS"-"<3>",
                  "a[NSt]|BY"-"<2>",
                  "<[^>]*>"-"|",
                  "\\|[^|][^|]*\\|"-"-"], Sequence, Replaced),
    string_length(Replaced, ReplacedLength),
    format('~n~d~n~d~n~d~n', [OriginalLength, CleanLength, ReplacedLength]).
