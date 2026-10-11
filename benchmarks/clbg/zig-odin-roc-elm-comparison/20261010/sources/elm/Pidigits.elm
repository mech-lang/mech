port module Pidigits exposing (main)
import Platform
import Round
import Integer as I
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
z n = I.fromSafeInt n
extract a q r t =
    I.divBy t (I.add (I.mul (z a) q) r) |> Maybe.withDefault (z 0)
spigot n produced k q r t pending rows =
    if produced == n then String.concat(List.reverse rows) else
        let
            first = extract 3 q r t
            second = extract 4 q r t
        in
        if I.compare first second == EQ then
            let
                digit = I.toInt first
                count = produced+1
                line = pending++String.fromInt digit
                complete = modBy 10 count == 0 || count == n
                nextRows = if complete then (String.padRight 10 ' ' line++"\t:"++String.fromInt count++"\n")::rows else rows
                nextPending = if complete then "" else line
                nextR = I.mul (z 10) (I.sub r (I.mul (z digit) t))
                nextQ = I.mul (z 10) q
            in
            spigot n count k nextQ nextR t nextPending nextRows
        else
            let
                nextK = k+1
                multiplier = z(2*nextK+1)
                nextR = I.mul multiplier (I.add r (I.mul (z 2) q))
                nextQ = I.mul (z nextK) q
                nextT = I.mul multiplier t
            in
            spigot n produced nextK nextQ nextR nextT pending rows
run n _ = spigot n 0 0 (z 1) (z 0) (z 1) "" []
