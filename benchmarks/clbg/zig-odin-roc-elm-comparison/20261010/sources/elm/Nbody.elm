port module Nbody exposing (main)
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
type alias Body =
    { x : Float, y : Float, z : Float, vx : Float, vy : Float, vz : Float, mass : Float }
solar = 4 * pi * pi
year = 365.24
body x y z vx vy vz mass =
    { x=x, y=y, z=z, vx=vx*year, vy=vy*year, vz=vz*year, mass=mass*solar }
bodies =
    [ body 0 0 0 0 0 0 1
    , body 4.84143144246472090 -1.16032004402742839 -0.103622044471123109 0.00166007664274403694 0.00769901118419740425 -0.0000690460016972063023 0.000954791938424326609
    , body 8.34336671824457987 4.12479856412430479 -0.403523417114321381 -0.00276742510726862411 0.00499852801234917238 0.0000230417297573763929 0.000285885980666130812
    , body 12.8943695621391310 -15.1111514016986312 -0.223307578892655734 0.00296460137564761618 0.00237847173959480950 -0.0000296589568540237556 0.0000436624404335156298
    , body 15.3796971148509165 -25.9193146099879641 0.179258772950371181 0.00268067772490389322 0.00162824170038242295 -0.0000951592254519715870 0.0000515138902046611451
    ]
offset xs =
    let
        momentum = List.foldl (\b p -> {x=p.x+b.vx*b.mass,y=p.y+b.vy*b.mass,z=p.z+b.vz*b.mass}) {x=0,y=0,z=0} xs
    in
    case xs of
        sun::planets -> {sun|vx= -momentum.x/solar,vy= -momentum.y/solar,vz= -momentum.z/solar}::planets
        [] -> []
interactions a rest =
    case rest of
        [] -> (a,[])
        b::tail ->
            let
                dx = a.x-b.x
                dy = a.y-b.y
                dz = a.z-b.z
                square = dx*dx+dy*dy+dz*dz
                magnitude = 0.01/(square*sqrt square)
                a1 = {a|vx=a.vx-dx*b.mass*magnitude,vy=a.vy-dy*b.mass*magnitude,vz=a.vz-dz*b.mass*magnitude}
                b1 = {b|vx=b.vx+dx*a.mass*magnitude,vy=b.vy+dy*a.mass*magnitude,vz=b.vz+dz*a.mass*magnitude}
                (a2,remaining) = interactions a1 tail
            in
            (a2,b1::remaining)
velocities xs =
    case xs of
        [] -> []
        a::rest ->
            let (updated,remaining) = interactions a rest in
            updated::velocities remaining
advance xs =
    List.map (\b -> {b|x=b.x+0.01*b.vx,y=b.y+0.01*b.vy,z=b.z+0.01*b.vz}) (velocities xs)
energy xs =
    case xs of
        [] -> 0
        a::rest ->
            let
                potential b =
                    let
                        dx = a.x-b.x
                        dy = a.y-b.y
                        dz = a.z-b.z
                    in
                    a.mass*b.mass/sqrt(dx*dx+dy*dy+dz*dz)
            in
            0.5*a.mass*(a.vx*a.vx+a.vy*a.vy+a.vz*a.vz)-List.sum(List.map potential rest)+energy rest
steps n xs =
    if n == 0 then xs else steps (n-1) (advance xs)
run n _ =
    let initial = offset bodies in
    Round.round 9 (energy initial)++"\n"++Round.round 9 (energy(steps n initial))++"\n"
