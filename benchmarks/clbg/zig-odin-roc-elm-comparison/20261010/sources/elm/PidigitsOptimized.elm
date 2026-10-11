port module PidigitsOptimized exposing (main)
import Platform
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
    I.divBy t (I.add (I.mul q (z a)) r) |> Maybe.withDefault (z 0)
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
                nextR = I.mul (I.sub r (I.mul t (z digit))) (z 10)
                nextQ = I.mul q (z 10)
            in
            spigot n count k nextQ nextR t nextPending nextRows
        else
            let
                nextK = k+1
                multiplier = z(2*nextK+1)
                nextR = I.mul (I.add r (I.mul q (z 2))) multiplier
                nextQ = I.mul q (z nextK)
                nextT = I.mul t multiplier
            in
            spigot n produced nextK nextQ nextR nextT pending rows
run n _ = spigot n 0 0 (z 1) (z 0) (z 1) "" []
