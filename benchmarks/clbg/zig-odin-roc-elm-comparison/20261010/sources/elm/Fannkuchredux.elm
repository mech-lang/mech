port module Fannkuchredux exposing (main)
import Platform
import Round
import Array
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
get i array = Array.get i array |> Maybe.withDefault 0
flips permutation total =
    case permutation of
        0::_ -> total
        first::_ ->
            flips (List.reverse(List.take (first+1) permutation)++List.drop (first+1) permutation) (total+1)
        [] -> total
rotate r permutation =
    case permutation of
        first::rest -> List.take r rest++[first]++List.drop r rest
        [] -> []
prepare r count =
    if r <= 1 then count else prepare (r-1) (Array.set (r-1) r count)
next n r permutation count =
    if r == n then Nothing else
        let
            rotated = rotate r permutation
            remaining = get r count-1
            updated = Array.set r remaining count
        in
        if remaining > 0 then Just(rotated,updated,r) else next n (r+1) rotated updated
enumerate n permutation count r index checksum maximum =
    let
        prepared = prepare r count
        current = flips permutation 0
        nextMaximum = max maximum current
        nextChecksum = checksum+(if modBy 2 index == 0 then current else -current)
    in
    case next n 1 permutation prepared of
        Nothing -> String.fromInt nextChecksum++"\nPfannkuchen("++String.fromInt n++") = "++String.fromInt nextMaximum++"\n"
        Just(perm,counts,rank) -> enumerate n perm counts rank (index+1) nextChecksum nextMaximum
run n _ = enumerate n (List.range 0 (n-1)) (Array.repeat n 0) n 0 0 0
