port module Fasta exposing (main)
import Platform
import Task

port input : ({ n : Int, input : String } -> msg) -> Sub msg
port output : { data : String, done : Bool } -> Cmd msg
type Msg = Start { n : Int, input : String } | Continue
continue = Task.perform (always Continue) (Task.succeed ())
send data = output { data=data, done=False }
main : Program () Model Msg
main = Platform.worker {init=\_ -> (initial,Cmd.none),update=update,subscriptions=\_ -> input Start}
type alias Model = {n:Int,stage:Int,offset:Int,seed:Int}
initial = {n=0,stage=0,offset=0,seed=42}
alu = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
cumulative pairs total =
    case pairs of
        [] -> []
        (char,probability)::rest -> (char,total+probability)::cumulative rest (total+probability)
iub = cumulative (List.map2 Tuple.pair (String.toList "acgtBDHKMNRSVWY") [0.27,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02]) 0
homo = cumulative (List.map2 Tuple.pair (String.toList "acgt") [0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008]) 0
choose value pairs =
    case pairs of
        (char,threshold)::rest -> if value < threshold then char else choose value rest
        [] -> 't'
randomLine remaining distribution seed chars =
    if remaining == 0 then (String.fromList(List.reverse chars),seed) else
        let
            next = modBy 139968 (seed*3877+29573)
            value = toFloat next/139968
            char = choose value distribution
        in
        randomLine (remaining-1) distribution next (char::chars)
header stage =
    case stage of
        0 -> ">ONE Homo sapiens alu\n"
        1 -> ">TWO IUB ambiguity codes\n"
        _ -> ">THREE Homo sapiens frequency\n"
block remaining model lines =
    let
        length = model.n*(if model.stage == 0 then 2 else if model.stage == 1 then 3 else 5)
        take = min 60 (length-model.offset)
    in
    if remaining == 0 || take == 0 then (model,String.concat(List.reverse lines)) else
        let
            (line,seed) =
                if model.stage == 0 then
                    let start = modBy (String.length alu) model.offset in
                    (String.slice start (start+take) (alu++alu),model.seed)
                else randomLine take (if model.stage == 1 then iub else homo) model.seed []
            next = {model|offset=model.offset+take,seed=seed}
        in
        block (remaining-1) next ((line++"\n")::lines)
update msg model =
    case msg of
        Start args -> ({initial|n=args.n},Cmd.batch[send(header 0),continue])
        Continue ->
            let
                (next,data) = block 1024 model []
                length = next.n*(if next.stage == 0 then 2 else if next.stage == 1 then 3 else 5)
                finished = next.offset == length
            in
            if finished && next.stage == 2 then (next,output{data=data,done=True}) else
                if finished then
                    let advanced = {next|stage=next.stage+1,offset=0} in
                    (advanced,Cmd.batch[send(data++header advanced.stage),continue])
                else (next,Cmd.batch[send data,continue])
