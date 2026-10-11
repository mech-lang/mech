port module Spectralnorm exposing (main)
import Platform
import Round

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
coefficient i j =
    1/toFloat(((i+j)*(i+j+1)//2)+i+1)
row transposed i j xs total =
    case xs of
        [] -> total
        x::rest ->
            let a = if transposed then coefficient j i else coefficient i j in
            row transposed i (j+1) rest (total+a*x)
multiply transposed n xs =
    List.map (\i -> row transposed i 0 xs 0) (List.range 0 (n-1))
ata n xs = multiply True n (multiply False n xs)
power k n u v =
    if k == 0 then (u,v) else
        let
            v1 = ata n u
            u1 = ata n v1
        in
        power (k-1) n u1 v1
dot xs ys total =
    case (xs,ys) of
        (x::xr,y::yr) -> dot xr yr (total+x*y)
        _ -> total
run n _ =
    let (u,v) = power 10 n (List.repeat n 1) (List.repeat n 0) in
    Round.round 9 (sqrt(dot u v 0/dot v v 0))++"\n"
