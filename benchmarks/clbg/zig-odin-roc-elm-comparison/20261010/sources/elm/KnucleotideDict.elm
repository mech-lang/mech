port module KnucleotideDict exposing (main)
import Platform
import Round
import Dict
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
code char =
    case char of
        'C' -> 1
        'G' -> 2
        'T' -> 3
        _ -> 0
empty = Dict.empty
increment previous = Just(Maybe.withDefault 0 previous+1)
count width sequence =
    let
        modulus = 4^width
        step char (key,seen,counts) =
            let
                next = modBy modulus (key*4+code char)
                updated = if seen+1 >= width then Dict.update next increment counts else counts
            in
            (next,seen+1,updated)
        (_,_,result) = String.foldl step (0,0,empty) sequence
    in
    result
encode text = String.foldl (\char key -> key*4+code char) 0 text
decode width key =
    if width == 0 then "" else
        decode (width-1) (floor(toFloat key/4))++String.slice (modBy 4 key) (modBy 4 key+1) "ACGT"
compareEntry (ka,ca) (kb,cb) =
    if ca == cb then compare ka kb else compare cb ca
frequencies width sequence =
    let
        counts = count width sequence
        denominator = toFloat(String.length sequence-width+1)
        row (key,total) = decode width key++" "++Round.round 3 (100*toFloat total/denominator)++"\n"
    in
    String.concat(List.map row (List.sortWith compareEntry (Dict.toList counts)))++"\n"
run _ raw =
    let
        start = String.indexes ">THREE" raw |> List.head |> Maybe.withDefault 0
        record = String.dropLeft start raw
        headerEnd = String.indexes "\n" record |> List.head |> Maybe.withDefault 0
        sequence = String.dropLeft (headerEnd+1) record |> String.split ">" |> List.head |> Maybe.withDefault "" |> String.replace "\n" "" |> String.replace "\r" "" |> String.toUpper
        queries = [(3,"GGT"),(4,"GGTA"),(6,"GGTATT"),(12,"GGTATTTTAATT"),(18,"GGTATTTTAATTTATAGT")]
        query (width,text) =
            let counts = count width sequence in
            String.fromInt(Dict.get (encode text) counts |> Maybe.withDefault 0)++"\t"++text++"\n"
    in
    frequencies 1 sequence++frequencies 2 sequence++String.concat(List.map query queries)
