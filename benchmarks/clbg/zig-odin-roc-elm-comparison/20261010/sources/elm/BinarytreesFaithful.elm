port module BinarytreesFaithful exposing (main)
import Platform

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
type Tree = Node Int Tree Tree | Empty
build depth identity =
    if depth == 0 then Node identity Empty Empty else
        Node identity (build (depth-1) (2*identity)) (build (depth-1) (2*identity+1))
check tree =
    case tree of
        Empty -> 0
        Node _ left right -> 1+check left+check right
temporary remaining depth total =
    if remaining == 0 then total else
        temporary (remaining-1) depth (total+check(build depth remaining))
depths depth maximum longLived rows =
    if depth > maximum then
        rows++"long lived tree of depth "++String.fromInt maximum++"\t check: "++String.fromInt(check longLived)++"\n"
    else
        let
            iterations = 2^(maximum-depth+4)
            total = temporary iterations depth 0
            line = String.fromInt iterations++"\t trees of depth "++String.fromInt depth++"\t check: "++String.fromInt total++"\n"
        in
        depths (depth+2) maximum longLived (rows++line)
run n _ =
    let
        maximum = max 6 n
        stretch = check(build (maximum+1) 1)
    in
    "stretch tree of depth "++String.fromInt(maximum+1)++"\t check: "++String.fromInt stretch++"\n"++depths 4 maximum (build maximum 1) ""
