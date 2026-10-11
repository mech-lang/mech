port module Regexredux exposing (main)
import Platform
import Round
import Regex
port input : ({ n : Int, input : String } -> msg) -> Sub msg
port output : { data : String, done : Bool } -> Cmd msg
type Msg = Start { n : Int, input : String }
main : Program () () Msg
main =
    Platform.worker
        { init = \_ -> ( (), Cmd.none )
        , update = \(Start args) _ -> ( (), output { data = run args.n args.input, done = True } )
        , subscriptions = \_ -> input Start
        }
compile pattern = Regex.fromString pattern |> Maybe.withDefault Regex.never
patterns =
    [ "agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]", "a[act]ggtaaa|tttacc[agt]t"
    , "ag[act]gtaaa|tttac[agt]ct", "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct"
    , "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct", "agggtaa[cgt]|[acg]ttaccct"
    ]
replacements =
    [ ("tHa[Nt]","<4>"), ("aND|caN|Ha[DS]|WaS","<3>"), ("a[NSt]|BY","<2>")
    , ("<[^>]*>","|"), ("\\|[^|][^|]*\\|","-")
    ]
run _ raw =
    let
        cleaned = Regex.replace (compile ">.*\n|\n") (always "") raw
        counts pattern =
            let re = Regex.fromStringWith {caseInsensitive=True,multiline=False} pattern |> Maybe.withDefault Regex.never in
            pattern++" "++String.fromInt(List.length(Regex.find re cleaned))++"\n"
        replaced = List.foldl (\(pattern,text) source -> Regex.replace (compile pattern) (always text) source) cleaned replacements
    in
    String.concat(List.map counts patterns)++"\n"++String.fromInt(String.length raw)++"\n"++String.fromInt(String.length cleaned)++"\n"++String.fromInt(String.length replaced)++"\n"
