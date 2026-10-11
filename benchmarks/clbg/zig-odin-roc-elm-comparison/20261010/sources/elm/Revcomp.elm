port module Revcomp exposing (main)
import Platform
import Task

port input : ({ n : Int, input : String } -> msg) -> Sub msg
port output : { data : String, done : Bool } -> Cmd msg
type Msg = Start { n : Int, input : String } | Continue
continue = Task.perform (always Continue) (Task.succeed ())
send data = output { data=data, done=False }
main : Program () Model Msg
main = Platform.worker {init=\_ -> (initial,Cmd.none),update=update,subscriptions=\_ -> input Start}
type alias Model = {records:List String,sequence:String,position:Int}
initial = {records=[],sequence="",position=0}
complement char =
    case Char.toUpper char of
        'A' -> 'T'
        'T' -> 'A'
        'U' -> 'A'
        'C' -> 'G'
        'G' -> 'C'
        'M' -> 'K'
        'R' -> 'Y'
        'W' -> 'W'
        'S' -> 'S'
        'Y' -> 'R'
        'K' -> 'M'
        'V' -> 'B'
        'H' -> 'D'
        'D' -> 'H'
        'B' -> 'V'
        _ -> 'N'
start records =
    case records of
        [] -> (initial,output{data="",done=True})
        record::rest ->
            let
                end = String.indexes "\n" record |> List.head |> Maybe.withDefault 0
                name = ">"++String.left end record++"\n"
                sequence = String.dropLeft (end+1) record |> String.replace "\n" "" |> String.replace "\r" "" |> String.reverse |> String.map complement
            in
            ({records=rest,sequence=sequence,position=0},Cmd.batch[send name,continue])
block left sequence position lines =
    if left == 0 || position >= String.length sequence then (position,String.concat(List.reverse lines)) else
        block (left-1) sequence (position+60) ((String.slice position (position+60) sequence++"\n")::lines)
update msg model =
    case msg of
        Start args -> start (List.drop 1 (String.split ">" args.input))
        Continue ->
            if model.position >= String.length model.sequence then start model.records else
                let (position,data) = block 1024 model.sequence model.position [] in
                ({model|position=position},Cmd.batch[send data,continue])
