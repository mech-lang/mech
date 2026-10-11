port module Mandelbrot exposing (main)
import Platform
import Task

port input : ({ n : Int, input : String } -> msg) -> Sub msg
port output : { data : String, done : Bool } -> Cmd msg
type Msg = Start { n : Int, input : String } | Continue
continue = Task.perform (always Continue) (Task.succeed ())
send data = output { data=data, done=False }
main : Program () Model Msg
main = Platform.worker {init=\_ -> (initial,Cmd.none),update=update,subscriptions=\_ -> input Start}
type alias Model = {n:Int,row:Int}
initial = {n=0,row=0}
inside remaining cr ci zr zi =
    if zr*zr+zi*zi > 4 then 0 else
        if remaining == 0 then 1 else
            inside (remaining-1) cr ci (zr*zr-zi*zi+cr) (2*zr*zi+ci)
byte n row x bit packed =
    if bit == 8 then Char.fromCode packed else
        let
            value = if x+bit < n then inside 50 (2*toFloat(x+bit)/toFloat n-1.5) (2*toFloat row/toFloat n-1) 0 0 else 0
        in
        byte n row x (bit+1) (packed*2+value)
pixels n row x bytes =
    if x >= n then String.fromList(List.reverse bytes) else pixels n row (x+8) (byte n row x 0 0::bytes)
update msg model =
    case msg of
        Start args -> ({n=args.n,row=0},Cmd.batch[send("P4\n"++String.fromInt args.n++" "++String.fromInt args.n++"\n"),continue])
        Continue ->
            let
                data = pixels model.n model.row 0 []
                finished = model.row+1 == model.n
                next = {model|row=model.row+1}
                command = output {data=data,done=finished}
            in
            (next,if finished then command else Cmd.batch[command,continue])
