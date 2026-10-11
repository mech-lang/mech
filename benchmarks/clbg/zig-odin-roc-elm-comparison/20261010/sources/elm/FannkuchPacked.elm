port module FannkuchPacked exposing (main)
import Platform
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
format n checksum maximum = String.fromInt checksum++"\nPfannkuchen("++String.fromInt n++") = "++String.fromInt maximum++"\n"
reverseDigits remaining value reversed place =
    if remaining == 0 then reversed+value*place else
        let
            quotient = floor(toFloat value/16)
            digit = value-quotient*16
        in
        reverseDigits (remaining-1) quotient (reversed*16+digit) (place*16)
packedFlips permutation total =
    let first = permutation-floor(toFloat permutation/16)*16 in
    if first == 0 then total else packedFlips (reverseDigits (first+1) permutation 0 1) (total+1)
preparePacked r count =
    if r <= 1 then count else
        let
            place = 16^(r-1)
            tail = floor(toFloat count/toFloat place)
            previous = tail-floor(toFloat tail/16)*16
        in
        preparePacked (r-1) (count+(r-previous)*place)
packedLoop n permutation count r index checksum maximum advancing =
    if advancing then
        if r == n then format n checksum maximum else
            let
                place = 16^r
                prefix = permutation-floor(toFloat permutation/toFloat(place*16))*(place*16)
                first = permutation-floor(toFloat permutation/16)*16
                rotated = permutation-prefix+floor(toFloat prefix/16)+first*place
                tail = floor(toFloat count/toFloat place)
                remaining = tail-floor(toFloat tail/16)*16-1
                updated = count-place
            in
            if remaining > 0 then packedLoop n rotated updated r index checksum maximum False else
                packedLoop n rotated updated (r+1) index checksum maximum True
    else
        let
            current = packedFlips permutation 0
            prepared = preparePacked r count
            nextChecksum = checksum+(if index == floor(toFloat index/2)*2 then current else -current)
        in
        packedLoop n permutation prepared 1 (index+1) nextChecksum (max maximum current) True
get i array = Array.get i array |> Maybe.withDefault 0
flips permutation total =
    case permutation of
        0::_ -> total
        first::_ -> flips (List.reverse(List.take (first+1) permutation)++List.drop (first+1) permutation) (total+1)
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
        Nothing -> format n nextChecksum nextMaximum
        Just(perm,counts,rank) -> enumerate n perm counts rank (index+1) nextChecksum nextMaximum
run n _ =
    if n <= 13 then
        let permutation = List.foldl (\i packed -> packed+i*16^i) 0 (List.range 0 (n-1)) in
        packedLoop n permutation 0 n 0 0 0 False
    else enumerate n (List.range 0 (n-1)) (Array.repeat n 0) n 0 0 0
