"""Pure Elm workloads. The process adapter performs only generic port I/O."""
import json
from bootstrap import D,sha

HEADER='''port module MODULE exposing (main)
import Platform
import Round
EXTRA
port input : ({ n : Int, input : String } -> msg) -> Sub msg
port output : { data : String, done : Bool } -> Cmd msg
type Msg = Start { n : Int, input : String }
main : Program () () Msg
main =
    Platform.worker
        { init = \\_ -> ( (), Cmd.none )
        , update = \\(Start args) _ -> ( (), output { data = run args.n args.input, done = True } )
        , subscriptions = \\_ -> input Start
        }
'''

NBODY='''type alias Body =
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
        momentum = List.foldl (\\b p -> {x=p.x+b.vx*b.mass,y=p.y+b.vy*b.mass,z=p.z+b.vz*b.mass}) {x=0,y=0,z=0} xs
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
    List.map (\\b -> {b|x=b.x+0.01*b.vx,y=b.y+0.01*b.vy,z=b.z+0.01*b.vz}) (velocities xs)
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
    Round.round 9 (energy initial)++"\\n"++Round.round 9 (energy(steps n initial))++"\\n"
'''

SPECTRAL='''coefficient i j =
    1/toFloat(((i+j)*(i+j+1)//2)+i+1)
row transposed i j xs total =
    case xs of
        [] -> total
        x::rest ->
            let a = if transposed then coefficient j i else coefficient i j in
            row transposed i (j+1) rest (total+a*x)
multiply transposed n xs =
    List.map (\\i -> row transposed i 0 xs 0) (List.range 0 (n-1))
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
    Round.round 9 (sqrt(dot u v 0/dot v v 0))++"\\n"
'''

TREE='''type Tree = Node Int Tree Tree | Empty
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
depths depth maximum =
    if depth > maximum then "" else
        let
            iterations = 2^(maximum-depth+4)
            total = temporary iterations depth 0
        in
        String.fromInt iterations++"\\t trees of depth "++String.fromInt depth++"\\t check: "++String.fromInt total++"\\n"++depths (depth+2) maximum
run n _ =
    let
        maximum = max 6 n
        stretch = check(build (maximum+1) 1)
        longLived = build maximum 1
        rows = depths 4 maximum
        retainedCheck = check longLived
    in
    "stretch tree of depth "++String.fromInt(maximum+1)++"\\t check: "++String.fromInt stretch++"\\n"++rows++"long lived tree of depth "++String.fromInt maximum++"\\t check: "++String.fromInt retainedCheck++"\\n"
'''

REGEX='''compile pattern = Regex.fromString pattern |> Maybe.withDefault Regex.never
patterns =
    [ "agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]", "a[act]ggtaaa|tttacc[agt]t"
    , "ag[act]gtaaa|tttac[agt]ct", "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct"
    , "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct", "agggtaa[cgt]|[acg]ttaccct"
    ]
replacements =
    [ ("tHa[Nt]","<4>"), ("aND|caN|Ha[DS]|WaS","<3>"), ("a[NSt]|BY","<2>")
    , ("<[^>]*>","|"), ("\\\\|[^|][^|]*\\\\|","-")
    ]
run _ raw =
    let
        cleaned = Regex.replace (compile ">.*\\n|\\n") (always "") raw
        counts pattern =
            let re = Regex.fromStringWith {caseInsensitive=True,multiline=False} pattern |> Maybe.withDefault Regex.never in
            pattern++" "++String.fromInt(List.length(Regex.find re cleaned))++"\\n"
        replaced = List.foldl (\\(pattern,text) source -> Regex.replace (compile pattern) (always text) source) cleaned replacements
    in
    String.concat(List.map counts patterns)++"\\n"++String.fromInt(String.length raw)++"\\n"++String.fromInt(String.length cleaned)++"\\n"++String.fromInt(String.length replaced)++"\\n"
'''

PI='''z n = I.fromSafeInt n
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
                nextRows = if complete then (String.padRight 10 ' ' line++"\\t:"++String.fromInt count++"\\n")::rows else rows
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
'''

KNUC='''code char =
    case char of
        'C' -> 1
        'G' -> 2
        'T' -> 3
        _ -> 0
hash key = Bitwise.xor (Bitwise.and 0x7FFFFFFF key) (floor(toFloat key/2147483648))
empty = H.empty Equality.int (Hashing.hash hash)
count width sequence =
    let
        modulus = 4^width
        step char (key,seen,counts) =
            let
                next = modBy modulus (key*4+code char)
                updated = if seen+1 >= width then H.update next (Maybe.withDefault 0 >> (+) 1 >> Just) counts else counts
            in
            (next,seen+1,updated)
        (_,_,result) = String.foldl step (0,0,empty) sequence
    in
    result
encode text = String.foldl (\\char key -> key*4+code char) 0 text
decode width key =
    if width == 0 then "" else
        decode (width-1) (floor(toFloat key/4))++String.slice (modBy 4 key) (modBy 4 key+1) "ACGT"
compareEntry (ka,ca) (kb,cb) =
    if ca == cb then compare ka kb else compare cb ca
frequencies width sequence =
    let
        counts = count width sequence
        denominator = toFloat(String.length sequence-width+1)
        row (key,total) = decode width key++" "++Round.round 3 (100*toFloat total/denominator)++"\\n"
    in
    String.concat(List.map row (List.sortWith compareEntry (H.toList counts)))++"\\n"
run _ raw =
    let
        start = String.indexes ">THREE" raw |> List.head |> Maybe.withDefault 0
        record = String.dropLeft start raw
        headerEnd = String.indexes "\\n" record |> List.head |> Maybe.withDefault 0
        sequence = String.dropLeft (headerEnd+1) record |> String.split ">" |> List.head |> Maybe.withDefault "" |> String.replace "\\n" "" |> String.replace "\\r" "" |> String.toUpper
        queries = [(3,"GGT"),(4,"GGTA"),(6,"GGTATT"),(12,"GGTATTTTAATT"),(18,"GGTATTTTAATTTATAGT")]
        query (width,text) =
            let counts = count width sequence in
            String.fromInt(H.get (encode text) counts |> Maybe.withDefault 0)++"\\t"++text++"\\n"
    in
    frequencies 1 sequence++frequencies 2 sequence++String.concat(List.map query queries)
'''

FANN='''get i array = Array.get i array |> Maybe.withDefault 0
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
        Nothing -> String.fromInt nextChecksum++"\\nPfannkuchen("++String.fromInt n++") = "++String.fromInt nextMaximum++"\\n"
        Just(perm,counts,rank) -> enumerate n perm counts rank (index+1) nextChecksum nextMaximum
run n _ = enumerate n (List.range 0 (n-1)) (Array.repeat n 0) n 0 0 0
'''

STREAM='''port module MODULE exposing (main)
import Platform
import Task
EXTRA
port input : ({ n : Int, input : String } -> msg) -> Sub msg
port output : { data : String, done : Bool } -> Cmd msg
type Msg = Start { n : Int, input : String } | Continue
continue = Task.perform (always Continue) (Task.succeed ())
send data = output { data=data, done=False }
main : Program () Model Msg
main = Platform.worker {init=\\_ -> (initial,Cmd.none),update=update,subscriptions=\\_ -> input Start}
'''

MANDEL='''type alias Model = {n:Int,row:Int}
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
        Start args -> ({n=args.n,row=0},Cmd.batch[send("P4\\n"++String.fromInt args.n++" "++String.fromInt args.n++"\\n"),continue])
        Continue ->
            let
                data = pixels model.n model.row 0 []
                finished = model.row+1 == model.n
                next = {model|row=model.row+1}
                command = output {data=data,done=finished}
            in
            (next,if finished then command else Cmd.batch[command,continue])
'''

ALU='GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA'
FASTA='''type alias Model = {n:Int,stage:Int,offset:Int,seed:Int}
initial = {n=0,stage=0,offset=0,seed=42}
alu = "ALU"
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
        0 -> ">ONE Homo sapiens alu\\n"
        1 -> ">TWO IUB ambiguity codes\\n"
        _ -> ">THREE Homo sapiens frequency\\n"
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
        block (remaining-1) next ((line++"\\n")::lines)
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
'''.replace('ALU',ALU)

REV='''type alias Model = {records:List String,sequence:String,position:Int}
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
                end = String.indexes "\\n" record |> List.head |> Maybe.withDefault 0
                name = ">"++String.left end record++"\\n"
                sequence = String.dropLeft (end+1) record |> String.replace "\\n" "" |> String.replace "\\r" "" |> String.reverse |> String.map complement
            in
            ({records=rest,sequence=sequence,position=0},Cmd.batch[send name,continue])
block left sequence position lines =
    if left == 0 || position >= String.length sequence then (position,String.concat(List.reverse lines)) else
        block (left-1) sequence (position+60) ((String.slice position (position+60) sequence++"\\n")::lines)
update msg model =
    case msg of
        Start args -> start (List.drop 1 (String.split ">" args.input))
        Continue ->
            if model.position >= String.length model.sequence then start model.records else
                let (position,data) = block 1024 model.sequence model.position [] in
                ({model|position=position},Cmd.batch[send data,continue])
'''

def main():
    rows=[]
    def emit(bench,name,extra,body,stream=False):
        source=(STREAM if stream else HEADER).replace('MODULE',name).replace('EXTRA',extra)+body
        path=D/'sources/elm'/(name+'.elm')
        path.write_text(source)
        identifier=bench+'-elm-'+('streamed' if stream else 'idiomatic')
        rows.append(dict(key=bench+':'+identifier,id=identifier,benchmark=bench,language='elm',source=str(path),source_sha256=sha(path),
            source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/'+bench+'.html',
            local_adaptation=True,adaptations=['Locally authored pure Elm workload, compiled with --optimize and executed by Node. Only generic port/process I/O is in the adapter. Every algorithm helper counts toward LOC.'],
            libraries=['elm/core','elm/json']+({'pidigits':['dwayne/elm-integer','dwayne/elm-natural'],'regexredux':['elm/regex'],'knucleotide':['nikita-volkov/hashing-containers','nikita-volkov/typeclasses']} .get(bench,[]))))
    emit('nbody','Nbody','',NBODY)
    emit('spectralnorm','Spectralnorm','',SPECTRAL)
    emit('binarytrees','Binarytrees','',TREE)
    emit('regexredux','Regexredux','import Regex',REGEX)
    emit('pidigits','Pidigits','import Integer as I',PI)
    emit('knucleotide','Knucleotide','import HashingContainers.HashDict as H\nimport Typeclasses.Classes.Equality as Equality\nimport Typeclasses.Classes.Hashing as Hashing\nimport Bitwise',KNUC)
    emit('fannkuchredux','Fannkuchredux','import Array',FANN)
    emit('mandelbrot','Mandelbrot','',MANDEL,True)
    emit('fasta','Fasta','',FASTA,True)
    emit('revcomp','Revcomp','',REV,True)
    (D/'written-elm.json').write_text(json.dumps(dict(programs=rows),indent=2)+'\n')
    print('WROTE ELM',len(rows),flush=True)

if __name__=='__main__': main()
