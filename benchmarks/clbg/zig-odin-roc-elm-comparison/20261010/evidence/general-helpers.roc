app [main!] {
    pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU.tar.br",
}
import pf.Stdout
import pf.Arg exposing [Arg]
get = |items, index|
    when List.get(items,index) is
        Ok(value) -> value
        Err(_) -> crash("index out of bounds")
Big : {negative : Bool, digits : List U64}

base = 4294967296u64
trim_digits = |digits|
    if List.is_empty(digits) then digits else
        if get(digits,List.len(digits)-1) == 0 then trim_digits(List.drop_last(digits,1)) else digits
big : Bool, List U64 -> Big
big = |negative, digits|
    normalized = trim_digits(digits)
    {negative:negative && !List.is_empty(normalized),digits:normalized}
natural = |value|
    if value == 0 then big(Bool.false,[]) else
        if value < base then big(Bool.false,[value]) else big(Bool.false,[value%base,value//base])
compare_digits = |a,b,remaining|
    if remaining == 0 then EQ else
        index = remaining-1
        order = Num.compare(get(a,index),get(b,index))
        if order == EQ then compare_digits(a,b,index) else order
compare_abs = |a,b|
    order = Num.compare(List.len(a),List.len(b))
    if order == EQ then compare_digits(a,b,List.len(a)) else order
add_digits = |a,b,index,carry,output|
    length = Num.max(List.len(a),List.len(b))
    if index >= length then
        if carry == 0 then output else List.append(output,carry)
    else
        av = List.get(a,index) ?? 0
        bv = List.get(b,index) ?? 0
        sum = av+bv+carry
        add_digits(a,b,index+1,sum//base,List.append(output,sum%base))
add_abs = |a,b| add_digits(a,b,0,0,List.with_capacity(Num.max(List.len(a),List.len(b))+1))
subtract_digits = |a,b,index,borrow,output|
    if index >= List.len(a) then trim_digits(output) else
        av = get(a,index)
        bv = (List.get(b,index) ?? 0)+borrow
        digit = if av < bv then av+base-bv else av-bv
        next_borrow = if av < bv then 1u64 else 0u64
        subtract_digits(a,b,index+1,next_borrow,List.append(output,digit))
subtract_abs = |a,b| subtract_digits(a,b,0,0,List.with_capacity(List.len(a)))
big_add : Big, Big -> Big
big_add = |a,b|
    if a.negative == b.negative then big(a.negative,add_abs(a.digits,b.digits)) else
        when compare_abs(a.digits,b.digits) is
            LT -> big(b.negative,subtract_abs(b.digits,a.digits))
            _ -> big(a.negative,subtract_abs(a.digits,b.digits))
big_sub = |a,b| big_add(a,big(!b.negative,b.digits))
multiply_digits = |digits,factor,index,carry,output|
    if index >= List.len(digits) then
        if carry == 0 then output else
            multiply_digits(digits,factor,index,carry//Num.to_u128(base),List.append(output,Num.to_u64(carry%Num.to_u128(base))))
    else
        product = Num.to_u128(get(digits,index))*Num.to_u128(factor)+carry
        digit = Num.to_u64(product%Num.to_u128(base))
        multiply_digits(digits,factor,index+1,product//Num.to_u128(base),List.append(output,digit))
big_mul = |a,factor| big(a.negative,multiply_digits(a.digits,factor,0,0u128,List.with_capacity(List.len(a.digits)+2)))
bit_length = |digits|
    if List.is_empty(digits) then 0u64 else
        top = get(digits,List.len(digits)-1)
        (List.len(digits)-1)*32+64-Num.to_u64(Num.count_leading_zero_bits(top))
shift_bits = |digits,bits|
    scale = Num.shift_left_by(1u64,Num.to_u8(bits%32))
    multiplied = big_mul(big(Bool.false,digits),scale)
    List.concat(List.repeat(0u64,bits//32),multiplied.digits)
power_bit = |bits| List.append(List.repeat(0u64,bits//32),Num.shift_left_by(1u64,Num.to_u8(bits%32)))
divide_abs = |remainder,divisor,quotient|
    if compare_abs(remainder,divisor) == LT then (quotient,remainder) else
        difference = bit_length(remainder)-bit_length(divisor)
        candidate = shift_bits(divisor,difference)
        bits = if compare_abs(candidate,remainder) == GT then difference-1 else difference
        scaled = if bits == difference then candidate else shift_bits(divisor,bits)
        next = subtract_abs(remainder,scaled)
        updated = add_abs(quotient,power_bit(bits))
        divide_abs(next,divisor,updated)
big_floor : Big, Big -> Big
big_floor = |a,b|
    if List.is_empty(b.digits) then crash("division by zero") else
        (quotient,remainder) = divide_abs(a.digits,b.digits,[])
        negative = a.negative != b.negative
        rounded = if negative && !List.is_empty(remainder) then add_abs(quotient,[1u64]) else quotient
        big(negative,rounded)
Pattern : [Literal U8, Dot, Class Bool (List U8), Sequence (List Pattern), Alternate (List Pattern), Repeat Pattern U64 U64, Begin, End]

escaped = |byte|
    when byte is
        110 -> 10u8
        114 -> 13u8
        116 -> 9u8
        _ -> byte
class_chars = |tokens,index,chars|
    if index >= List.len(tokens) then crash("unterminated character class") else
        value = get(tokens,index)
        if value == 93 then (chars,index+1) else
            (byte,next) = if value == 92 then (escaped(get(tokens,index+1)),index+2) else (value,index+1)
            if next+1 < List.len(tokens) && get(tokens,next) == 45 && get(tokens,next+1) != 93 then
                last = get(tokens,next+1)
                expanded = List.range({start:At(byte),end:At(last)})
                class_chars(tokens,next+2,List.concat(chars,expanded))
            else class_chars(tokens,next,List.append(chars,byte))
parse_alt : List U8, U64 -> (Pattern,U64)
parse_alt = |tokens,index|
    (sequence,next) = parse_sequence(tokens,index,[])
    if next < List.len(tokens) && get(tokens,next) == 124 then
        (rest,after) = parse_alt(tokens,next+1)
        (Alternate([sequence,rest]),after)
    else (sequence,next)
parse_sequence : List U8, U64, List Pattern -> (Pattern,U64)
parse_sequence = |tokens,index,parts|
    if index >= List.len(tokens) then (Sequence(parts),index) else
        token = get(tokens,index)
        if token == 124 || token == 41 then (Sequence(parts),index) else
            (atom,next) = parse_atom(tokens,index)
            (piece,after) = if next < List.len(tokens) then
                when get(tokens,next) is
                    42 -> (Repeat(atom,0,Num.max_u64),next+1)
                    43 -> (Repeat(atom,1,Num.max_u64),next+1)
                    63 -> (Repeat(atom,0,1),next+1)
                    _ -> (atom,next)
            else (atom,next)
            parse_sequence(tokens,after,List.append(parts,piece))
parse_atom : List U8, U64 -> (Pattern,U64)
parse_atom = |tokens,index|
    token = get(tokens,index)
    when token is
        40 ->
            (group,next) = parse_alt(tokens,index+1)
            if next >= List.len(tokens) || get(tokens,next) != 41 then crash("unterminated group") else (group,next+1)
        91 ->
            negated = index+1 < List.len(tokens) && get(tokens,index+1) == 94
            start = if negated then index+2 else index+1
            (chars,next) = class_chars(tokens,start,[])
            (Class(negated,chars),next)
        92 -> (Literal(escaped(get(tokens,index+1))),index+2)
        46 -> (Dot,index+1)
        94 -> (Begin,index+1)
        36 -> (End,index+1)
        _ -> (Literal(token),index+1)
compile = |text|
    tokens = Str.to_utf8(text)
    (pattern,index) = parse_alt(tokens,0)
    if index != List.len(tokens) then crash("unexpected regex token") else pattern
fold_ascii = |byte| if byte >= 65 && byte <= 90 then byte+32 else byte
same_byte = |a,b,insensitive| if insensitive then fold_ascii(a) == fold_ascii(b) else a == b
atom_matches = |pattern,byte,insensitive|
    when pattern is
        Literal(value) -> same_byte(value,byte,insensitive)
        Dot -> byte != 10
        Class(negated,chars) -> negated != List.any(chars,|value| same_byte(value,byte,insensitive))
        _ -> Bool.false
nullable = |pattern|
    when pattern is
        Sequence(parts) -> List.all(parts,nullable)
        Alternate(parts) -> List.any(parts,nullable)
        Repeat(piece,minimum,_) -> minimum == 0 || nullable(piece)
        Begin | End -> Bool.true
        _ -> Bool.false
sequence_starts = |parts,byte,insensitive|
    when parts is
        [] -> Bool.true
        [head,.. as rest] -> can_start(head,byte,insensitive) || (nullable(head) && sequence_starts(rest,byte,insensitive))
can_start = |pattern,byte,insensitive|
    when pattern is
        Sequence(parts) -> sequence_starts(parts,byte,insensitive)
        Alternate(parts) -> List.any(parts,|p| can_start(p,byte,insensitive))
        Repeat(piece,minimum,_) -> minimum == 0 || can_start(piece,byte,insensitive)
        Begin | End -> Bool.true
        _ -> atom_matches(pattern,byte,insensitive)
match_sequence : List Pattern, List U8, U64, Bool -> List U64
match_sequence = |parts,input,position,insensitive|
    when parts is
        [] -> [position]
        [piece,.. as rest] ->
            endings = match_pattern(piece,input,position,insensitive)
            List.join_map(endings,|next| match_sequence(rest,input,next,insensitive))
match_repeat : Pattern, List U8, U64, Bool, U64, U64, U64 -> List U64
match_repeat = |piece,input,position,insensitive,count,minimum,maximum|
    current = if count >= minimum then [position] else []
    if count == maximum then current else
        endings = List.keep_if(match_pattern(piece,input,position,insensitive),|next| next > position)
        advanced = List.join_map(endings,|next| match_repeat(piece,input,next,insensitive,count+1,minimum,maximum))
        List.concat(advanced,current)
match_pattern : Pattern, List U8, U64, Bool -> List U64
match_pattern = |pattern,input,position,insensitive|
    when pattern is
        Sequence(parts) -> match_sequence(parts,input,position,insensitive)
        Alternate(parts) -> List.join_map(parts,|piece| match_pattern(piece,input,position,insensitive))
        Repeat(piece,minimum,maximum) -> match_repeat(piece,input,position,insensitive,0,minimum,maximum)
        Begin -> if position == 0 then [position] else []
        End -> if position == List.len(input) then [position] else []
        _ ->
            if position < List.len(input) && atom_matches(pattern,get(input,position),insensitive) then [position+1] else []
find_next = |pattern,input,position,insensitive|
    if position > List.len(input) then NotFound else
        possible = if position == List.len(input) then nullable(pattern) else can_start(pattern,get(input,position),insensitive)
        ends = if possible then match_pattern(pattern,input,position,insensitive) else []
        when List.first(ends) is
            Ok(last) -> Found(position,last)
            Err(_) -> if position == List.len(input) then NotFound else find_next(pattern,input,position+1,insensitive)
count_matches = |pattern,input,position,insensitive,total|
    when find_next(pattern,input,position,insensitive) is
        NotFound -> total
        Found(first,last) -> count_matches(pattern,input,if first == last then last+1 else last,insensitive,total+1)
replace_from = |pattern,input,replacement,position,output|
    if position > List.len(input) then output else
        when find_next(pattern,input,position,Bool.false) is
            NotFound -> List.concat(output,List.sublist(input,{start:position,len:List.len(input)-position}))
            Found(first,last) ->
                before = List.sublist(input,{start:position,len:first-position})
                joined = List.concat(List.concat(output,before),replacement)
                if first == last && last < List.len(input) then
                    replace_from(pattern,input,replacement,last+1,List.append(joined,get(input,last)))
                else replace_from(pattern,input,replacement,if first == last then last+1 else last,joined)
replace_regex = |pattern,input,replacement| replace_from(pattern,input,Str.to_utf8(replacement),0,List.with_capacity(List.len(input)))
expect big_add(big(Bool.false,[1001405880u64,2843664299u64,3440937117u64,181162775u64,3152692833u64,871966u64]),big(Bool.false,[1414705937u64,1724006503u64,1u64])) == big(Bool.false,[2416111817u64,272703506u64,3440937119u64,181162775u64,3152692833u64,871966u64])
expect big_sub(big(Bool.false,[1001405880u64,2843664299u64,3440937117u64,181162775u64,3152692833u64,871966u64]),big(Bool.false,[1414705937u64,1724006503u64,1u64])) == big(Bool.false,[3881667239u64,1119657795u64,3440937116u64,181162775u64,3152692833u64,871966u64])
expect big_floor(big(Bool.false,[1001405880u64,2843664299u64,3440937117u64,181162775u64,3152692833u64,871966u64]),big(Bool.false,[1414705937u64,1724006503u64,1u64])) == big(Bool.false,[236114958u64,281439367u64,2082369642u64,622210u64])
expect big_mul(big(Bool.false,[1001405880u64,2843664299u64,3440937117u64,181162775u64,3152692833u64,871966u64]),4680992476990748819u64) == big(Bool.false,[1199288488u64,2502539805u64,1403926996u64,2617868317u64,3844420718u64,3356237466u64,3259928508u64,221267u64])
expect big_add(big(Bool.true,[714331976u64,1686731348u64,25651464u64]),big(Bool.true,[11031750u64,3374939663u64,2424000875u64,1076186u64])) == big(Bool.true,[725363726u64,766703715u64,2449652340u64,1076186u64])
expect big_sub(big(Bool.true,[714331976u64,1686731348u64,25651464u64]),big(Bool.true,[11031750u64,3374939663u64,2424000875u64,1076186u64])) == big(Bool.false,[3591667070u64,1688208314u64,2398349411u64,1076186u64])
expect big_floor(big(Bool.true,[714331976u64,1686731348u64,25651464u64]),big(Bool.true,[11031750u64,3374939663u64,2424000875u64,1076186u64])) == big(Bool.false,[])
expect big_mul(big(Bool.true,[714331976u64,1686731348u64,25651464u64]),6042946083962661367u64) == big(Bool.true,[1186152056u64,3286935801u64,1074617121u64,2171158917u64,8403131u64])
expect big_add(big(Bool.false,[2598006504u64,3358505735u64,731833846u64,825297179u64,1482262725u64,1248920918u64,1284396512u64]),big(Bool.true,[11402744u64])) == big(Bool.false,[2586603760u64,3358505735u64,731833846u64,825297179u64,1482262725u64,1248920918u64,1284396512u64])
expect big_sub(big(Bool.false,[2598006504u64,3358505735u64,731833846u64,825297179u64,1482262725u64,1248920918u64,1284396512u64]),big(Bool.true,[11402744u64])) == big(Bool.false,[2609409248u64,3358505735u64,731833846u64,825297179u64,1482262725u64,1248920918u64,1284396512u64])
expect big_floor(big(Bool.false,[2598006504u64,3358505735u64,731833846u64,825297179u64,1482262725u64,1248920918u64,1284396512u64]),big(Bool.true,[11402744u64])) == big(Bool.true,[1660112980u64,3254846514u64,3995194662u64,1300732416u64,3691998937u64,2745550381u64,112u64])
expect big_mul(big(Bool.false,[2598006504u64,3358505735u64,731833846u64,825297179u64,1482262725u64,1248920918u64,1284396512u64]),1570509694601206947u64) == big(Bool.false,[1642986936u64,3730383598u64,668504838u64,1450540980u64,3112924793u64,1361855978u64,2375395953u64,2272567033u64,109350309u64])
expect big_add(big(Bool.true,[1368519506u64,3787910876u64,3704165431u64,3793487953u64,201u64]),big(Bool.false,[2891494250u64,248564740u64,1861u64])) == big(Bool.true,[2771992552u64,3539346135u64,3704163570u64,3793487953u64,201u64])
expect big_sub(big(Bool.true,[1368519506u64,3787910876u64,3704165431u64,3793487953u64,201u64]),big(Bool.false,[2891494250u64,248564740u64,1861u64])) == big(Bool.true,[4260013756u64,4036475616u64,3704167292u64,3793487953u64,201u64])
expect big_floor(big(Bool.true,[1368519506u64,3787910876u64,3704165431u64,3793487953u64,201u64]),big(Bool.false,[2891494250u64,248564740u64,1861u64])) == big(Bool.true,[1884370776u64,465908087u64])
expect big_mul(big(Bool.true,[1368519506u64,3787910876u64,3704165431u64,3793487953u64,201u64]),2506739105417500776u64) == big(Bool.true,[2815265104u64,3440689632u64,3464581719u64,3404930062u64,3528678578u64,1864163915u64,27u64])
expect big_add(big(Bool.false,[2409599037u64,2334804554u64,4208481459u64,167974199u64,2345215733u64,1798856765u64,368u64]),big(Bool.true,[916086056u64,3389918680u64,294380517u64,3451182806u64,75415205u64])) == big(Bool.false,[1493512981u64,3239853170u64,3914100941u64,1011758689u64,2269800527u64,1798856765u64,368u64])
expect big_sub(big(Bool.false,[2409599037u64,2334804554u64,4208481459u64,167974199u64,2345215733u64,1798856765u64,368u64]),big(Bool.true,[916086056u64,3389918680u64,294380517u64,3451182806u64,75415205u64])) == big(Bool.false,[3325685093u64,1429755938u64,207894681u64,3619157006u64,2420630938u64,1798856765u64,368u64])
expect big_floor(big(Bool.false,[2409599037u64,2334804554u64,4208481459u64,167974199u64,2345215733u64,1798856765u64,368u64]),big(Bool.true,[916086056u64,3389918680u64,294380517u64,3451182806u64,75415205u64])) == big(Bool.true,[3439195257u64,20981u64])
expect big_mul(big(Bool.false,[2409599037u64,2334804554u64,4208481459u64,167974199u64,2345215733u64,1798856765u64,368u64]),14373683192280969975u64) == big(Bool.false,[1637209307u64,3092483157u64,3513985297u64,1778639474u64,405957227u64,1776801726u64,1578670990u64,307331945u64,287u64])
expect big_add(big(Bool.true,[2608546275u64,2540140512u64,3751221048u64,4080349281u64,363982453u64]),big(Bool.true,[1795015650u64,2234530782u64,901557434u64,2902389144u64,144498587u64])) == big(Bool.true,[108594629u64,479703999u64,357811187u64,2687771130u64,508481041u64])
expect big_sub(big(Bool.true,[2608546275u64,2540140512u64,3751221048u64,4080349281u64,363982453u64]),big(Bool.true,[1795015650u64,2234530782u64,901557434u64,2902389144u64,144498587u64])) == big(Bool.true,[813530625u64,305609730u64,2849663614u64,1177960137u64,219483866u64])
expect big_floor(big(Bool.true,[2608546275u64,2540140512u64,3751221048u64,4080349281u64,363982453u64]),big(Bool.true,[1795015650u64,2234530782u64,901557434u64,2902389144u64,144498587u64])) == big(Bool.false,[2u64])
expect big_mul(big(Bool.true,[2608546275u64,2540140512u64,3751221048u64,4080349281u64,363982453u64]),7424648112320970279u64) == big(Bool.true,[947045269u64,1591246465u64,1199212885u64,2057748530u64,2506335689u64,3588629774u64,146499654u64])
expect big_add(big(Bool.false,[430237334u64,1352757428u64,50275273u64,2095895364u64,2084151414u64,2324562861u64,390991467u64]),big(Bool.false,[687961909u64,113802007u64,164u64])) == big(Bool.false,[1118199243u64,1466559435u64,50275437u64,2095895364u64,2084151414u64,2324562861u64,390991467u64])
expect big_sub(big(Bool.false,[430237334u64,1352757428u64,50275273u64,2095895364u64,2084151414u64,2324562861u64,390991467u64]),big(Bool.false,[687961909u64,113802007u64,164u64])) == big(Bool.false,[4037242721u64,1238955420u64,50275109u64,2095895364u64,2084151414u64,2324562861u64,390991467u64])
expect big_floor(big(Bool.false,[430237334u64,1352757428u64,50275273u64,2095895364u64,2084151414u64,2324562861u64,390991467u64]),big(Bool.false,[687961909u64,113802007u64,164u64])) == big(Bool.false,[50542790u64,3641610463u64,1406934080u64,821417767u64,2383709u64])
expect big_mul(big(Bool.false,[430237334u64,1352757428u64,50275273u64,2095895364u64,2084151414u64,2324562861u64,390991467u64]),9771993252330718324u64) == big(Bool.false,[1338856440u64,3469009815u64,67027437u64,2214118284u64,2721314187u64,2962635517u64,1878446092u64,4041492959u64,207124138u64])
expect big_add(big(Bool.true,[1773040864u64,156u64]),big(Bool.true,[1815005833u64,3138370313u64,2109055868u64,2436398614u64,547u64])) == big(Bool.true,[3588046697u64,3138370469u64,2109055868u64,2436398614u64,547u64])
expect big_sub(big(Bool.true,[1773040864u64,156u64]),big(Bool.true,[1815005833u64,3138370313u64,2109055868u64,2436398614u64,547u64])) == big(Bool.false,[41964969u64,3138370157u64,2109055868u64,2436398614u64,547u64])
expect big_floor(big(Bool.true,[1773040864u64,156u64]),big(Bool.true,[1815005833u64,3138370313u64,2109055868u64,2436398614u64,547u64])) == big(Bool.false,[])
expect big_mul(big(Bool.true,[1773040864u64,156u64]),3965836790465474540u64) == big(Bool.true,[2298011264u64,3947500579u64,2692722404u64,33u64])
expect big_add(big(Bool.false,[1762079691u64,2770602420u64,1286308774u64]),big(Bool.true,[1041552250u64,331120359u64,27462889u64])) == big(Bool.false,[720527441u64,2439482061u64,1258845885u64])
expect big_sub(big(Bool.false,[1762079691u64,2770602420u64,1286308774u64]),big(Bool.true,[1041552250u64,331120359u64,27462889u64])) == big(Bool.false,[2803631941u64,3101722779u64,1313771663u64])
expect big_floor(big(Bool.false,[1762079691u64,2770602420u64,1286308774u64]),big(Bool.true,[1041552250u64,331120359u64,27462889u64])) == big(Bool.true,[47u64])
expect big_mul(big(Bool.false,[1762079691u64,2770602420u64,1286308774u64]),4953656864881154299u64) == big(Bool.false,[1625182217u64,208151703u64,4261840207u64,2960111722u64,345423141u64])
expect big_add(big(Bool.true,[222411206u64,4144713455u64,131788885u64,2263730266u64,84u64]),big(Bool.false,[970933787u64,81u64])) == big(Bool.true,[3546444715u64,4144713373u64,131788885u64,2263730266u64,84u64])
expect big_sub(big(Bool.true,[222411206u64,4144713455u64,131788885u64,2263730266u64,84u64]),big(Bool.false,[970933787u64,81u64])) == big(Bool.true,[1193344993u64,4144713536u64,131788885u64,2263730266u64,84u64])
expect big_floor(big(Bool.true,[222411206u64,4144713455u64,131788885u64,2263730266u64,84u64]),big(Bool.false,[970933787u64,81u64])) == big(Bool.true,[2865253227u64,3214302059u64,174546171u64,1u64])
expect big_mul(big(Bool.true,[222411206u64,4144713455u64,131788885u64,2263730266u64,84u64]),4785151934451651420u64) == big(Bool.true,[639464744u64,3764910072u64,3643010238u64,3592606489u64,1179579333u64,3979826988u64,21u64])
expect big_add(big(Bool.false,[3153u64]),big(Bool.true,[623782527u64,14388503u64])) == big(Bool.true,[623779374u64,14388503u64])
expect big_sub(big(Bool.false,[3153u64]),big(Bool.true,[623782527u64,14388503u64])) == big(Bool.false,[623785680u64,14388503u64])
expect big_floor(big(Bool.false,[3153u64]),big(Bool.true,[623782527u64,14388503u64])) == big(Bool.true,[1u64])
expect big_mul(big(Bool.false,[3153u64]),16192681515307348420u64) == big(Bool.false,[529166084u64,3116197374u64,2767u64])
expect big_add(big(Bool.true,[2870267362u64,55575037u64]),big(Bool.true,[157516163u64,2228446853u64,3408378635u64,797412540u64,57u64])) == big(Bool.true,[3027783525u64,2284021890u64,3408378635u64,797412540u64,57u64])
expect big_sub(big(Bool.true,[2870267362u64,55575037u64]),big(Bool.true,[157516163u64,2228446853u64,3408378635u64,797412540u64,57u64])) == big(Bool.false,[1582216097u64,2172871815u64,3408378635u64,797412540u64,57u64])
expect big_floor(big(Bool.true,[2870267362u64,55575037u64]),big(Bool.true,[157516163u64,2228446853u64,3408378635u64,797412540u64,57u64])) == big(Bool.false,[])
expect big_mul(big(Bool.true,[2870267362u64,55575037u64]),7316194196916460088u64) == big(Bool.true,[1586724208u64,2250600875u64,747840075u64,22041709u64])
expect big_add(big(Bool.false,[3317526475u64,2355210589u64,11u64]),big(Bool.false,[2438179999u64,18u64])) == big(Bool.false,[1460739178u64,2355210608u64,11u64])
expect big_sub(big(Bool.false,[3317526475u64,2355210589u64,11u64]),big(Bool.false,[2438179999u64,18u64])) == big(Bool.false,[879346476u64,2355210571u64,11u64])
expect big_floor(big(Bool.false,[3317526475u64,2355210589u64,11u64]),big(Bool.false,[2438179999u64,18u64])) == big(Bool.false,[2671299956u64])
expect big_mul(big(Bool.false,[3317526475u64,2355210589u64,11u64]),566311595327685527u64) == big(Bool.false,[459755965u64,2847575093u64,2787226092u64,1522706150u64])
expect big_add(big(Bool.true,[519782u64]),big(Bool.true,[28u64])) == big(Bool.true,[519810u64])
expect big_sub(big(Bool.true,[519782u64]),big(Bool.true,[28u64])) == big(Bool.true,[519754u64])
expect big_floor(big(Bool.true,[519782u64]),big(Bool.true,[28u64])) == big(Bool.false,[18563u64])
expect big_mul(big(Bool.true,[519782u64]),15029836419103987158u64) == big(Bool.true,[1127720772u64,1263080762u64,423502u64])
expect big_add(big(Bool.false,[2162361899u64,463637482u64,42962630u64,4189400666u64,615615210u64,599727806u64,2696u64]),big(Bool.true,[1u64])) == big(Bool.false,[2162361898u64,463637482u64,42962630u64,4189400666u64,615615210u64,599727806u64,2696u64])
expect big_sub(big(Bool.false,[2162361899u64,463637482u64,42962630u64,4189400666u64,615615210u64,599727806u64,2696u64]),big(Bool.true,[1u64])) == big(Bool.false,[2162361900u64,463637482u64,42962630u64,4189400666u64,615615210u64,599727806u64,2696u64])
expect big_floor(big(Bool.false,[2162361899u64,463637482u64,42962630u64,4189400666u64,615615210u64,599727806u64,2696u64]),big(Bool.true,[1u64])) == big(Bool.true,[2162361899u64,463637482u64,42962630u64,4189400666u64,615615210u64,599727806u64,2696u64])
expect big_mul(big(Bool.false,[2162361899u64,463637482u64,42962630u64,4189400666u64,615615210u64,599727806u64,2696u64]),10523457064380789156u64) == big(Bool.false,[238907020u64,1418106648u64,1785773441u64,1303700582u64,3737852996u64,1412159457u64,2463214101u64,376557756u64,1538u64])
expect big_add(big(Bool.true,[3278141260u64,2663175498u64,165591409u64,3898212955u64,2588552054u64,2461031363u64,631156u64]),big(Bool.false,[3387928549u64,2526927737u64,975251628u64,60949356u64])) == big(Bool.true,[4185180007u64,136247760u64,3485307077u64,3837263598u64,2588552054u64,2461031363u64,631156u64])
expect big_sub(big(Bool.true,[3278141260u64,2663175498u64,165591409u64,3898212955u64,2588552054u64,2461031363u64,631156u64]),big(Bool.false,[3387928549u64,2526927737u64,975251628u64,60949356u64])) == big(Bool.true,[2371102513u64,895135940u64,1140843038u64,3959162311u64,2588552054u64,2461031363u64,631156u64])
expect big_floor(big(Bool.true,[3278141260u64,2663175498u64,165591409u64,3898212955u64,2588552054u64,2461031363u64,631156u64]),big(Bool.false,[3387928549u64,2526927737u64,975251628u64,60949356u64])) == big(Bool.true,[3092894687u64,3251394439u64,44476217u64])
expect big_mul(big(Bool.true,[3278141260u64,2663175498u64,165591409u64,3898212955u64,2588552054u64,2461031363u64,631156u64]),8270148384078866132u64) == big(Bool.true,[2525369072u64,2971067907u64,758554177u64,3575260752u64,275576598u64,3661427543u64,3573820344u64,2903164529u64,282963u64])
expect big_add(big(Bool.false,[482195503u64,4137760853u64,3195867357u64,619598987u64,2222879166u64,73136u64]),big(Bool.true,[3006484104u64,249432u64])) == big(Bool.false,[1770678695u64,4137511420u64,3195867357u64,619598987u64,2222879166u64,73136u64])
expect big_sub(big(Bool.false,[482195503u64,4137760853u64,3195867357u64,619598987u64,2222879166u64,73136u64]),big(Bool.true,[3006484104u64,249432u64])) == big(Bool.false,[3488679607u64,4138010285u64,3195867357u64,619598987u64,2222879166u64,73136u64])
expect big_floor(big(Bool.false,[482195503u64,4137760853u64,3195867357u64,619598987u64,2222879166u64,73136u64]),big(Bool.true,[3006484104u64,249432u64])) == big(Bool.true,[3285853326u64,3592504031u64,2966419175u64,1259333483u64])
expect big_mul(big(Bool.false,[482195503u64,4137760853u64,3195867357u64,619598987u64,2222879166u64,73136u64]),8947245023333019703u64) == big(Bool.false,[908206617u64,818648184u64,510683912u64,4114302666u64,561055116u64,3076169951u64,2093190722u64,35473u64])
expect big_add(big(Bool.true,[3202316083u64,863995967u64,4208509818u64,3021344853u64,3999232594u64,2473774391u64,3210u64]),big(Bool.true,[4165808722u64,1511445162u64,3280u64])) == big(Bool.true,[3073157509u64,2375441130u64,4208513098u64,3021344853u64,3999232594u64,2473774391u64,3210u64])
expect big_sub(big(Bool.true,[3202316083u64,863995967u64,4208509818u64,3021344853u64,3999232594u64,2473774391u64,3210u64]),big(Bool.true,[4165808722u64,1511445162u64,3280u64])) == big(Bool.true,[3331474657u64,3647518100u64,4208506537u64,3021344853u64,3999232594u64,2473774391u64,3210u64])
expect big_floor(big(Bool.true,[3202316083u64,863995967u64,4208509818u64,3021344853u64,3999232594u64,2473774391u64,3210u64]),big(Bool.true,[4165808722u64,1511445162u64,3280u64])) == big(Bool.false,[2253719323u64,2219257414u64,1706327997u64,4203609603u64])
expect big_mul(big(Bool.true,[3202316083u64,863995967u64,4208509818u64,3021344853u64,3999232594u64,2473774391u64,3210u64]),8387258409578945132u64) == big(Bool.true,[2256680836u64,1766711489u64,2770457532u64,1245703051u64,316590277u64,1183425789u64,4131292908u64,3290061021u64,1459u64])
expect big_add(big(Bool.false,[4280568352u64,408278080u64]),big(Bool.false,[3130640009u64,86132730u64])) == big(Bool.false,[3116241065u64,494410811u64])
expect big_sub(big(Bool.false,[4280568352u64,408278080u64]),big(Bool.false,[3130640009u64,86132730u64])) == big(Bool.false,[1149928343u64,322145350u64])
expect big_floor(big(Bool.false,[4280568352u64,408278080u64]),big(Bool.false,[3130640009u64,86132730u64])) == big(Bool.false,[4u64])
expect big_mul(big(Bool.false,[4280568352u64,408278080u64]),7612197146481735094u64) == big(Bool.false,[2319798976u64,3885905632u64,1301073980u64,168479230u64])
expect big_add(big(Bool.true,[1787351181u64,1609057004u64,3886228535u64,42385404u64,794603383u64,3u64]),big(Bool.true,[84993979u64,3078355249u64,256584042u64])) == big(Bool.true,[1872345160u64,392444957u64,4142812578u64,42385404u64,794603383u64,3u64])
expect big_sub(big(Bool.true,[1787351181u64,1609057004u64,3886228535u64,42385404u64,794603383u64,3u64]),big(Bool.true,[84993979u64,3078355249u64,256584042u64])) == big(Bool.true,[1702357202u64,2825669051u64,3629644492u64,42385404u64,794603383u64,3u64])
expect big_floor(big(Bool.true,[1787351181u64,1609057004u64,3886228535u64,42385404u64,794603383u64,3u64]),big(Bool.true,[84993979u64,3078355249u64,256584042u64])) == big(Bool.false,[2868120446u64,1348345505u64,53u64])
expect big_mul(big(Bool.true,[1787351181u64,1609057004u64,3886228535u64,42385404u64,794603383u64,3u64]),12139562628807993729u64) == big(Bool.true,[235449357u64,1575939565u64,273998414u64,2726757275u64,1059149363u64,430039151u64,412370113u64,2u64])
expect big_add(big(Bool.false,[3022033059u64,3757399092u64,3267463451u64,199783301u64,372886702u64,1039381663u64,1342978623u64,1u64]),big(Bool.true,[3516966573u64,916036277u64,1447669787u64,1134289899u64,29047656u64])) == big(Bool.false,[3800033782u64,2841362814u64,1819793664u64,3360460698u64,343839045u64,1039381663u64,1342978623u64,1u64])
expect big_sub(big(Bool.false,[3022033059u64,3757399092u64,3267463451u64,199783301u64,372886702u64,1039381663u64,1342978623u64,1u64]),big(Bool.true,[3516966573u64,916036277u64,1447669787u64,1134289899u64,29047656u64])) == big(Bool.false,[2244032336u64,378468074u64,420165943u64,1334073201u64,401934358u64,1039381663u64,1342978623u64,1u64])
expect big_floor(big(Bool.false,[3022033059u64,3757399092u64,3267463451u64,199783301u64,372886702u64,1039381663u64,1342978623u64,1u64]),big(Bool.true,[3516966573u64,916036277u64,1447669787u64,1134289899u64,29047656u64])) == big(Bool.true,[982485826u64,399309527u64,194u64])
expect big_mul(big(Bool.false,[3022033059u64,3757399092u64,3267463451u64,199783301u64,372886702u64,1039381663u64,1342978623u64,1u64]),8734092819638281117u64) == big(Bool.false,[3185818871u64,2105779909u64,1527423321u64,72155762u64,965977277u64,351172202u64,3966148888u64,1540785608u64,2669432761u64])
expect big_add(big(Bool.true,[65727926u64]),big(Bool.false,[1063577185u64,3236879028u64,208022591u64])) == big(Bool.false,[997849259u64,3236879028u64,208022591u64])
expect big_sub(big(Bool.true,[65727926u64]),big(Bool.false,[1063577185u64,3236879028u64,208022591u64])) == big(Bool.true,[1129305111u64,3236879028u64,208022591u64])
expect big_floor(big(Bool.true,[65727926u64]),big(Bool.false,[1063577185u64,3236879028u64,208022591u64])) == big(Bool.true,[1u64])
expect big_mul(big(Bool.true,[65727926u64]),2026354225577637061u64) == big(Bool.true,[4123583758u64,996269524u64,7220139u64])
expect big_add(big(Bool.false,[128781706u64,2206358600u64,1160708827u64,3208317163u64,647466465u64,1315351895u64,302894u64]),big(Bool.true,[3971208739u64,1973776982u64,57296859u64])) == big(Bool.false,[452540263u64,232581617u64,1103411968u64,3208317163u64,647466465u64,1315351895u64,302894u64])
expect big_sub(big(Bool.false,[128781706u64,2206358600u64,1160708827u64,3208317163u64,647466465u64,1315351895u64,302894u64]),big(Bool.true,[3971208739u64,1973776982u64,57296859u64])) == big(Bool.false,[4099990445u64,4180135582u64,1218005686u64,3208317163u64,647466465u64,1315351895u64,302894u64])
expect big_floor(big(Bool.false,[128781706u64,2206358600u64,1160708827u64,3208317163u64,647466465u64,1315351895u64,302894u64]),big(Bool.true,[3971208739u64,1973776982u64,57296859u64])) == big(Bool.true,[1156736757u64,896273176u64,1016031110u64,22704929u64])
expect big_mul(big(Bool.false,[128781706u64,2206358600u64,1160708827u64,3208317163u64,647466465u64,1315351895u64,302894u64]),3480796759786866583u64) == big(Bool.false,[1570128486u64,2222431875u64,3374417596u64,1212119368u64,1206340045u64,1483933008u64,856456408u64,1934584885u64,57154u64])
expect big_add(big(Bool.true,[749811u64,2159541403u64,1089463726u64,2595942247u64,1631823598u64,4108087651u64,9191235u64]),big(Bool.true,[398512658u64,3118298742u64,394548902u64,2875342379u64,3547505729u64,159u64])) == big(Bool.true,[399262469u64,982872849u64,1484012629u64,1176317330u64,884362032u64,4108087811u64,9191235u64])
expect big_sub(big(Bool.true,[749811u64,2159541403u64,1089463726u64,2595942247u64,1631823598u64,4108087651u64,9191235u64]),big(Bool.true,[398512658u64,3118298742u64,394548902u64,2875342379u64,3547505729u64,159u64])) == big(Bool.true,[3897204449u64,3336209956u64,694914823u64,4015567164u64,2379285164u64,4108087491u64,9191235u64])
expect big_floor(big(Bool.true,[749811u64,2159541403u64,1089463726u64,2595942247u64,1631823598u64,4108087651u64,9191235u64]),big(Bool.true,[398512658u64,3118298742u64,394548902u64,2875342379u64,3547505729u64,159u64])) == big(Bool.false,[3332553953u64,57507u64])
expect big_mul(big(Bool.true,[749811u64,2159541403u64,1089463726u64,2595942247u64,1631823598u64,4108087651u64,9191235u64]),13581001754310043696u64) == big(Bool.true,[472442256u64,2780887274u64,740905435u64,2893171956u64,2722068315u64,2231024281u64,1006043346u64,1754347622u64,6766841u64])
expect big_add(big(Bool.false,[3463410326u64,3173038921u64,492255010u64,2782153359u64,4095020494u64,168311860u64,2352649862u64,215u64]),big(Bool.false,[898182u64])) == big(Bool.false,[3464308508u64,3173038921u64,492255010u64,2782153359u64,4095020494u64,168311860u64,2352649862u64,215u64])
expect big_sub(big(Bool.false,[3463410326u64,3173038921u64,492255010u64,2782153359u64,4095020494u64,168311860u64,2352649862u64,215u64]),big(Bool.false,[898182u64])) == big(Bool.false,[3462512144u64,3173038921u64,492255010u64,2782153359u64,4095020494u64,168311860u64,2352649862u64,215u64])
expect big_floor(big(Bool.false,[3463410326u64,3173038921u64,492255010u64,2782153359u64,4095020494u64,168311860u64,2352649862u64,215u64]),big(Bool.false,[898182u64])) == big(Bool.false,[4164967126u64,3984150889u64,4260093846u64,644289793u64,2956141269u64,287819450u64,1030716u64])
expect big_mul(big(Bool.false,[3463410326u64,3173038921u64,492255010u64,2782153359u64,4095020494u64,168311860u64,2352649862u64,215u64]),10087083984082530508u64) == big(Bool.false,[2891902856u64,49446770u64,209542846u64,3326226312u64,2550460437u64,1297320834u64,2269303028u64,231929702u64,3720492049u64,117u64])
expect big_add(big(Bool.true,[36u64]),big(Bool.true,[1645285865u64,4156875498u64,2862702488u64,851749887u64])) == big(Bool.true,[1645285901u64,4156875498u64,2862702488u64,851749887u64])
expect big_sub(big(Bool.true,[36u64]),big(Bool.true,[1645285865u64,4156875498u64,2862702488u64,851749887u64])) == big(Bool.false,[1645285829u64,4156875498u64,2862702488u64,851749887u64])
expect big_floor(big(Bool.true,[36u64]),big(Bool.true,[1645285865u64,4156875498u64,2862702488u64,851749887u64])) == big(Bool.false,[])
expect big_mul(big(Bool.true,[36u64]),15773814319474847859u64) == big(Bool.true,[2913521708u64,3365565391u64,30u64])
expect big_add(big(Bool.false,[1639126411u64,505442944u64,383807526u64,3492804214u64,4015629287u64,115699u64]),big(Bool.true,[4091193u64])) == big(Bool.false,[1635035218u64,505442944u64,383807526u64,3492804214u64,4015629287u64,115699u64])
expect big_sub(big(Bool.false,[1639126411u64,505442944u64,383807526u64,3492804214u64,4015629287u64,115699u64]),big(Bool.true,[4091193u64])) == big(Bool.false,[1643217604u64,505442944u64,383807526u64,3492804214u64,4015629287u64,115699u64])
expect big_floor(big(Bool.false,[1639126411u64,505442944u64,383807526u64,3492804214u64,4015629287u64,115699u64]),big(Bool.true,[4091193u64])) == big(Bool.true,[2332941562u64,211818841u64,375056613u64,3037135400u64,121462721u64])
expect big_mul(big(Bool.false,[1639126411u64,505442944u64,383807526u64,3492804214u64,4015629287u64,115699u64]),11849811888403446511u64) == big(Bool.false,[3046038213u64,225707573u64,3644150994u64,2955936991u64,1472200224u64,3200542910u64,1188601503u64,74323u64])
expect big_add(big(Bool.true,[1364517293u64,2040201082u64,1159500309u64,644097363u64,941675773u64,316228831u64,3105191169u64,5u64]),big(Bool.false,[17254630u64,923632155u64,1482861175u64,3468228785u64,282575u64])) == big(Bool.true,[1347262663u64,1116568927u64,3971606430u64,1470835873u64,941393197u64,316228831u64,3105191169u64,5u64])
expect big_sub(big(Bool.true,[1364517293u64,2040201082u64,1159500309u64,644097363u64,941675773u64,316228831u64,3105191169u64,5u64]),big(Bool.false,[17254630u64,923632155u64,1482861175u64,3468228785u64,282575u64])) == big(Bool.true,[1381771923u64,2963833237u64,2642361484u64,4112326148u64,941958348u64,316228831u64,3105191169u64,5u64])
expect big_floor(big(Bool.true,[1364517293u64,2040201082u64,1159500309u64,644097363u64,941675773u64,316228831u64,3105191169u64,5u64]),big(Bool.false,[17254630u64,923632155u64,1482861175u64,3468228785u64,282575u64])) == big(Bool.true,[2834985188u64,2599586925u64,86985u64])
expect big_mul(big(Bool.true,[1364517293u64,2040201082u64,1159500309u64,644097363u64,941675773u64,316228831u64,3105191169u64,5u64]),11857354279533229701u64) == big(Bool.true,[1900592353u64,2852090597u64,2697624082u64,3762996653u64,3184181896u64,4293743295u64,3275136585u64,386656819u64,2914856317u64,3u64])
expect big_add(big(Bool.false,[2966757415u64,15593504u64]),big(Bool.true,[3298243067u64,29382435u64])) == big(Bool.true,[331485652u64,13788931u64])
expect big_sub(big(Bool.false,[2966757415u64,15593504u64]),big(Bool.true,[3298243067u64,29382435u64])) == big(Bool.false,[1970033186u64,44975940u64])
expect big_floor(big(Bool.false,[2966757415u64,15593504u64]),big(Bool.true,[3298243067u64,29382435u64])) == big(Bool.true,[1u64])
expect big_mul(big(Bool.false,[2966757415u64,15593504u64]),4045780721754254549u64) == big(Bool.false,[2130277491u64,3942702937u64,4070717467u64,3420001u64])
expect big_add(big(Bool.true,[3124280632u64,2262647701u64,843399719u64,4035102005u64,1469252u64]),big(Bool.true,[3450655965u64,3517273538u64,1651019020u64,2734075595u64])) == big(Bool.true,[2279969301u64,1484953944u64,2494418740u64,2474210304u64,1469253u64])
expect big_sub(big(Bool.true,[3124280632u64,2262647701u64,843399719u64,4035102005u64,1469252u64]),big(Bool.true,[3450655965u64,3517273538u64,1651019020u64,2734075595u64])) == big(Bool.true,[3968591963u64,3040341458u64,3487347994u64,1301026409u64,1469252u64])
expect big_floor(big(Bool.true,[3124280632u64,2262647701u64,843399719u64,4035102005u64,1469252u64]),big(Bool.true,[3450655965u64,3517273538u64,1651019020u64,2734075595u64])) == big(Bool.false,[2308053u64])
expect big_mul(big(Bool.true,[3124280632u64,2262647701u64,843399719u64,4035102005u64,1469252u64]),12632196306394400000u64) == big(Bool.true,[2890143744u64,2942589860u64,1400436994u64,555885402u64,1055340864u64,3166575743u64,1006133u64])
expect big_add(big(Bool.false,[394667937u64,2407741133u64,2713735375u64,517u64]),big(Bool.false,[4235340086u64,899566944u64,518432u64])) == big(Bool.false,[335040727u64,3307308078u64,2714253807u64,517u64])
expect big_sub(big(Bool.false,[394667937u64,2407741133u64,2713735375u64,517u64]),big(Bool.false,[4235340086u64,899566944u64,518432u64])) == big(Bool.false,[454295147u64,1508174188u64,2713216943u64,517u64])
expect big_floor(big(Bool.false,[394667937u64,2407741133u64,2713735375u64,517u64]),big(Bool.false,[4235340086u64,899566944u64,518432u64])) == big(Bool.false,[4288336u64])
expect big_mul(big(Bool.false,[394667937u64,2407741133u64,2713735375u64,517u64]),4972796349431400062u64) == big(Bool.false,[642058046u64,220344632u64,2992194307u64,1811120407u64,2323720093u64,139u64])
expect big_add(big(Bool.true,[1020928634u64,3781627090u64,61u64]),big(Bool.true,[3097831173u64,3555719844u64,925326944u64])) == big(Bool.true,[4118759807u64,3042379638u64,925327006u64])
expect big_sub(big(Bool.true,[1020928634u64,3781627090u64,61u64]),big(Bool.true,[3097831173u64,3555719844u64,925326944u64])) == big(Bool.false,[2076902539u64,4069060050u64,925326882u64])
expect big_floor(big(Bool.true,[1020928634u64,3781627090u64,61u64]),big(Bool.true,[3097831173u64,3555719844u64,925326944u64])) == big(Bool.false,[])
expect big_mul(big(Bool.true,[1020928634u64,3781627090u64,61u64]),14633956575853572275u64) == big(Bool.true,[3374807886u64,675306692u64,1246752194u64,387844194u64,49u64])
expect big_add(big(Bool.false,[307u64]),big(Bool.true,[124435309u64,3510774702u64,2278606771u64,496261u64])) == big(Bool.true,[124435002u64,3510774702u64,2278606771u64,496261u64])
expect big_sub(big(Bool.false,[307u64]),big(Bool.true,[124435309u64,3510774702u64,2278606771u64,496261u64])) == big(Bool.false,[124435616u64,3510774702u64,2278606771u64,496261u64])
expect big_floor(big(Bool.false,[307u64]),big(Bool.true,[124435309u64,3510774702u64,2278606771u64,496261u64])) == big(Bool.true,[1u64])
expect big_mul(big(Bool.false,[307u64]),17540062993823187533u64) == big(Bool.false,[1935245911u64,3910812934u64,291u64])
expect big_add(big(Bool.true,[3450565228u64,2148543308u64,2741071924u64,932185309u64,2371065434u64,4013783071u64,2250192u64]),big(Bool.false,[1807745681u64,36498970u64,3730746572u64,833u64])) == big(Bool.true,[1642819547u64,2112044338u64,3305292648u64,932184475u64,2371065434u64,4013783071u64,2250192u64])
expect big_sub(big(Bool.true,[3450565228u64,2148543308u64,2741071924u64,932185309u64,2371065434u64,4013783071u64,2250192u64]),big(Bool.false,[1807745681u64,36498970u64,3730746572u64,833u64])) == big(Bool.true,[963343613u64,2185042279u64,2176851200u64,932186143u64,2371065434u64,4013783071u64,2250192u64])
expect big_floor(big(Bool.true,[3450565228u64,2148543308u64,2741071924u64,932185309u64,2371065434u64,4013783071u64,2250192u64]),big(Bool.false,[1807745681u64,36498970u64,3730746572u64,833u64])) == big(Bool.true,[3266254290u64,2174545951u64,2139400938u64,2698u64])
expect big_mul(big(Bool.true,[3450565228u64,2148543308u64,2741071924u64,932185309u64,2371065434u64,4013783071u64,2250192u64]),17770170872919656894u64) == big(Bool.true,[3508713512u64,803438700u64,2356657902u64,3610866659u64,2439336549u64,1763938735u64,3848874285u64,1581236570u64,2167662u64])
expect big_add(big(Bool.false,[986051268u64,3392901488u64,2040254382u64,3866411796u64,1238725914u64,736057663u64,4156130651u64,240195871u64]),big(Bool.true,[790085173u64,3358255074u64,3383599409u64,74272u64])) == big(Bool.false,[195966095u64,34646414u64,2951622269u64,3866337523u64,1238725914u64,736057663u64,4156130651u64,240195871u64])
expect big_sub(big(Bool.false,[986051268u64,3392901488u64,2040254382u64,3866411796u64,1238725914u64,736057663u64,4156130651u64,240195871u64]),big(Bool.true,[790085173u64,3358255074u64,3383599409u64,74272u64])) == big(Bool.false,[1776136441u64,2456189266u64,1128886496u64,3866486069u64,1238725914u64,736057663u64,4156130651u64,240195871u64])
expect big_floor(big(Bool.false,[986051268u64,3392901488u64,2040254382u64,3866411796u64,1238725914u64,736057663u64,4156130651u64,240195871u64]),big(Bool.true,[790085173u64,3358255074u64,3383599409u64,74272u64])) == big(Bool.true,[752423914u64,104389457u64,1203029150u64,4160589323u64,3233u64])
expect big_mul(big(Bool.false,[986051268u64,3392901488u64,2040254382u64,3866411796u64,1238725914u64,736057663u64,4156130651u64,240195871u64]),14194787588901207941u64) == big(Bool.false,[4254489556u64,1130725257u64,367699202u64,2317980740u64,1394536066u64,3026884141u64,1812123464u64,2550242354u64,742145715u64,184830958u64])
expect big_add(big(Bool.true,[1969098266u64,3466446948u64,2607052522u64,2678804875u64,1884100926u64,2798580152u64,862u64]),big(Bool.true,[4005441116u64,3273480764u64,1986432006u64,33u64])) == big(Bool.true,[1679572086u64,2444960417u64,298517233u64,2678804909u64,1884100926u64,2798580152u64,862u64])
expect big_sub(big(Bool.true,[1969098266u64,3466446948u64,2607052522u64,2678804875u64,1884100926u64,2798580152u64,862u64]),big(Bool.true,[4005441116u64,3273480764u64,1986432006u64,33u64])) == big(Bool.true,[2258624446u64,192966183u64,620620516u64,2678804842u64,1884100926u64,2798580152u64,862u64])
expect big_floor(big(Bool.true,[1969098266u64,3466446948u64,2607052522u64,2678804875u64,1884100926u64,2798580152u64,862u64]),big(Bool.true,[4005441116u64,3273480764u64,1986432006u64,33u64])) == big(Bool.false,[3918852834u64,2243080668u64,3348571159u64,25u64])
expect big_mul(big(Bool.true,[1969098266u64,3466446948u64,2607052522u64,2678804875u64,1884100926u64,2798580152u64,862u64]),14894352786334601226u64) == big(Bool.true,[3396709636u64,2703819693u64,1984193035u64,3218398491u64,3756898432u64,444032284u64,1357536289u64,2259229210u64,696u64])
expect big_add(big(Bool.false,[1365864342u64,1356657125u64,244659701u64]),big(Bool.false,[3716094873u64,264481010u64])) == big(Bool.false,[786991919u64,1621138136u64,244659701u64])
expect big_sub(big(Bool.false,[1365864342u64,1356657125u64,244659701u64]),big(Bool.false,[3716094873u64,264481010u64])) == big(Bool.false,[1944736765u64,1092176114u64,244659701u64])
expect big_floor(big(Bool.false,[1365864342u64,1356657125u64,244659701u64]),big(Bool.false,[3716094873u64,264481010u64])) == big(Bool.false,[3973084541u64])
expect big_mul(big(Bool.false,[1365864342u64,1356657125u64,244659701u64]),11588070824239840719u64) == big(Bool.false,[440662090u64,127890836u64,146609666u64,919986151u64,153692919u64])
expect big_add(big(Bool.true,[2951776198u64,1492844882u64,1631u64]),big(Bool.true,[1832733106u64,3178784783u64,1271722749u64,389343423u64])) == big(Bool.true,[489542008u64,376662370u64,1271724381u64,389343423u64])
expect big_sub(big(Bool.true,[2951776198u64,1492844882u64,1631u64]),big(Bool.true,[1832733106u64,3178784783u64,1271722749u64,389343423u64])) == big(Bool.false,[3175924204u64,1685939900u64,1271721118u64,389343423u64])
expect big_floor(big(Bool.true,[2951776198u64,1492844882u64,1631u64]),big(Bool.true,[1832733106u64,3178784783u64,1271722749u64,389343423u64])) == big(Bool.false,[])
expect big_mul(big(Bool.true,[2951776198u64,1492844882u64,1631u64]),28554955813704157u64) == big(Bool.true,[3392592878u64,3440140181u64,2436874556u64,2256028800u64,2u64])
expect big_add(big(Bool.false,[961869460u64,1894640512u64,13u64]),big(Bool.true,[2865564192u64,4109781361u64,1294358386u64,8734932u64])) == big(Bool.true,[1903694732u64,2215140849u64,1294358373u64,8734932u64])
expect big_sub(big(Bool.false,[961869460u64,1894640512u64,13u64]),big(Bool.true,[2865564192u64,4109781361u64,1294358386u64,8734932u64])) == big(Bool.false,[3827433652u64,1709454577u64,1294358400u64,8734932u64])
expect big_floor(big(Bool.false,[961869460u64,1894640512u64,13u64]),big(Bool.true,[2865564192u64,4109781361u64,1294358386u64,8734932u64])) == big(Bool.true,[1u64])
expect big_mul(big(Bool.false,[961869460u64,1894640512u64,13u64]),5941912936094091171u64) == big(Bool.false,[3119647804u64,4225573568u64,2869295791u64,1415389153u64,4u64])
expect big_add(big(Bool.true,[2934168886u64,820963817u64,1075008792u64,1398111u64]),big(Bool.false,[4039490520u64,1625054699u64,299176u64])) == big(Bool.true,[3189645662u64,3490876413u64,1074709615u64,1398111u64])
expect big_sub(big(Bool.true,[2934168886u64,820963817u64,1075008792u64,1398111u64]),big(Bool.false,[4039490520u64,1625054699u64,299176u64])) == big(Bool.true,[2678692110u64,2446018517u64,1075307968u64,1398111u64])
expect big_floor(big(Bool.true,[2934168886u64,820963817u64,1075008792u64,1398111u64]),big(Bool.false,[4039490520u64,1625054699u64,299176u64])) == big(Bool.true,[2891374840u64,4u64])
expect big_mul(big(Bool.true,[2934168886u64,820963817u64,1075008792u64,1398111u64]),693289735881102684u64) == big(Bool.true,[3898404200u64,3345553396u64,4020570976u64,537572269u64,2796784123u64,52545u64])
expect big_add(big(Bool.false,[4237201666u64,766648872u64,268234628u64,717293132u64,1111462884u64,2031479099u64,3u64]),big(Bool.true,[61u64])) == big(Bool.false,[4237201605u64,766648872u64,268234628u64,717293132u64,1111462884u64,2031479099u64,3u64])
expect big_sub(big(Bool.false,[4237201666u64,766648872u64,268234628u64,717293132u64,1111462884u64,2031479099u64,3u64]),big(Bool.true,[61u64])) == big(Bool.false,[4237201727u64,766648872u64,268234628u64,717293132u64,1111462884u64,2031479099u64,3u64])
expect big_floor(big(Bool.false,[4237201666u64,766648872u64,268234628u64,717293132u64,1111462884u64,2031479099u64,3u64]),big(Bool.true,[61u64])) == big(Bool.true,[1759285521u64,2054437712u64,1623811187u64,2616903001u64,3679504299u64,244530835u64])
expect big_mul(big(Bool.false,[4237201666u64,766648872u64,268234628u64,717293132u64,1111462884u64,2031479099u64,3u64]),10251461560964582946u64) == big(Bool.false,[270032452u64,1125647040u64,1455400601u64,4144262096u64,4065624497u64,717833407u64,3581661482u64,3994555543u64,1u64])
expect big_add(big(Bool.true,[3449404200u64,22415611u64,2421309182u64,2521169554u64,691502771u64,90675688u64]),big(Bool.true,[717984494u64,2099705021u64,125002u64])) == big(Bool.true,[4167388694u64,2122120632u64,2421434184u64,2521169554u64,691502771u64,90675688u64])
expect big_sub(big(Bool.true,[3449404200u64,22415611u64,2421309182u64,2521169554u64,691502771u64,90675688u64]),big(Bool.true,[717984494u64,2099705021u64,125002u64])) == big(Bool.true,[2731419706u64,2217677886u64,2421184179u64,2521169554u64,691502771u64,90675688u64])
expect big_floor(big(Bool.true,[3449404200u64,22415611u64,2421309182u64,2521169554u64,691502771u64,90675688u64]),big(Bool.true,[717984494u64,2099705021u64,125002u64])) == big(Bool.false,[1289108350u64,714147391u64,1679598598u64,725u64])
expect big_mul(big(Bool.true,[3449404200u64,22415611u64,2421309182u64,2521169554u64,691502771u64,90675688u64]),7740369290593794194u64) == big(Bool.true,[2378136784u64,930509402u64,1577967060u64,1476609205u64,3763836263u64,2431702733u64,1655052371u64,38048086u64])
expect big_add(big(Bool.false,[1929108215u64,1775369391u64,2861704006u64,3123087508u64,2833598837u64,3047019875u64,2258521413u64,233497u64]),big(Bool.false,[3667186504u64,14074u64])) == big(Bool.false,[1301327423u64,1775383466u64,2861704006u64,3123087508u64,2833598837u64,3047019875u64,2258521413u64,233497u64])
expect big_sub(big(Bool.false,[1929108215u64,1775369391u64,2861704006u64,3123087508u64,2833598837u64,3047019875u64,2258521413u64,233497u64]),big(Bool.false,[3667186504u64,14074u64])) == big(Bool.false,[2556889007u64,1775355316u64,2861704006u64,3123087508u64,2833598837u64,3047019875u64,2258521413u64,233497u64])
expect big_floor(big(Bool.false,[1929108215u64,1775369391u64,2861704006u64,3123087508u64,2833598837u64,3047019875u64,2258521413u64,233497u64]),big(Bool.false,[3667186504u64,14074u64])) == big(Bool.false,[1053646342u64,3765141382u64,3631788039u64,3847333647u64,3434700411u64,2532718782u64,16u64])
expect big_mul(big(Bool.false,[1929108215u64,1775369391u64,2861704006u64,3123087508u64,2833598837u64,3047019875u64,2258521413u64,233497u64]),2060052047706932884u64) == big(Bool.false,[4211024076u64,2931425300u64,2008898961u64,59178834u64,184609008u64,3293197893u64,1782570854u64,2669111293u64,4238576159u64,26075u64])
expect big_add(big(Bool.true,[505504019u64,1190176531u64,173145732u64]),big(Bool.true,[942651594u64,1792866232u64,2730886497u64,61128057u64,2377851671u64,2378u64])) == big(Bool.true,[1448155613u64,2983042763u64,2904032229u64,61128057u64,2377851671u64,2378u64])
expect big_sub(big(Bool.true,[505504019u64,1190176531u64,173145732u64]),big(Bool.true,[942651594u64,1792866232u64,2730886497u64,61128057u64,2377851671u64,2378u64])) == big(Bool.false,[437147575u64,602689701u64,2557740765u64,61128057u64,2377851671u64,2378u64])
expect big_floor(big(Bool.true,[505504019u64,1190176531u64,173145732u64]),big(Bool.true,[942651594u64,1792866232u64,2730886497u64,61128057u64,2377851671u64,2378u64])) == big(Bool.false,[])
expect big_mul(big(Bool.true,[505504019u64,1190176531u64,173145732u64]),14934576047731332641u64) == big(Bool.true,[4219004275u64,3285352370u64,2824895655u64,3500359934u64,140179648u64])
expect big_add(big(Bool.false,[]),big(Bool.true,[1004275485u64,4151522260u64,560181421u64,17u64])) == big(Bool.true,[1004275485u64,4151522260u64,560181421u64,17u64])
expect big_sub(big(Bool.false,[]),big(Bool.true,[1004275485u64,4151522260u64,560181421u64,17u64])) == big(Bool.false,[1004275485u64,4151522260u64,560181421u64,17u64])
expect big_floor(big(Bool.false,[]),big(Bool.true,[1004275485u64,4151522260u64,560181421u64,17u64])) == big(Bool.false,[])
expect big_mul(big(Bool.false,[]),14933492065046151139u64) == big(Bool.false,[])
expect big_add(big(Bool.true,[2706711098u64,1588911135u64,129148422u64,597024244u64,2594643132u64,12170843u64]),big(Bool.false,[18013u64])) == big(Bool.true,[2706693085u64,1588911135u64,129148422u64,597024244u64,2594643132u64,12170843u64])
expect big_sub(big(Bool.true,[2706711098u64,1588911135u64,129148422u64,597024244u64,2594643132u64,12170843u64]),big(Bool.false,[18013u64])) == big(Bool.true,[2706729111u64,1588911135u64,129148422u64,597024244u64,2594643132u64,12170843u64])
expect big_floor(big(Bool.true,[2706711098u64,1588911135u64,129148422u64,597024244u64,2594643132u64,12170843u64]),big(Bool.false,[18013u64])) == big(Bool.true,[2510654382u64,208720662u64,3282570583u64,3419221009u64,2877602840u64,675u64])
expect big_mul(big(Bool.true,[2706711098u64,1588911135u64,129148422u64,597024244u64,2594643132u64,12170843u64]),10471117179131072196u64) == big(Bool.true,[3619119208u64,3344183947u64,2326061197u64,467957548u64,513855488u64,4186836211u64,2268184168u64,6908662u64])
expect big_add(big(Bool.false,[1989958949u64,124963367u64,3788631217u64,1682252992u64,3522222249u64,2798651364u64,2586436111u64]),big(Bool.true,[272764233u64,1928183892u64,1594000268u64,3424338u64])) == big(Bool.false,[1717194716u64,2491746771u64,2194630948u64,1678828654u64,3522222249u64,2798651364u64,2586436111u64])
expect big_sub(big(Bool.false,[1989958949u64,124963367u64,3788631217u64,1682252992u64,3522222249u64,2798651364u64,2586436111u64]),big(Bool.true,[272764233u64,1928183892u64,1594000268u64,3424338u64])) == big(Bool.false,[2262723182u64,2053147259u64,1087664189u64,1685677331u64,3522222249u64,2798651364u64,2586436111u64])
expect big_floor(big(Bool.false,[1989958949u64,124963367u64,3788631217u64,1682252992u64,3522222249u64,2798651364u64,2586436111u64]),big(Bool.true,[272764233u64,1928183892u64,1594000268u64,3424338u64])) == big(Bool.true,[3142836623u64,1208384743u64,1330306714u64,755u64])
expect big_mul(big(Bool.false,[1989958949u64,124963367u64,3788631217u64,1682252992u64,3522222249u64,2798651364u64,2586436111u64]),13081238356141154148u64) == big(Bool.false,[1051057524u64,1522314603u64,1426603571u64,1597762144u64,3252234806u64,3677594512u64,681748076u64,203017858u64,1834133283u64])
expect big_add(big(Bool.true,[590682916u64,2901854080u64,1954437u64]),big(Bool.true,[1465456035u64,3832110007u64,2127590384u64,112546216u64])) == big(Bool.true,[2056138951u64,2438996791u64,2129544822u64,112546216u64])
expect big_sub(big(Bool.true,[590682916u64,2901854080u64,1954437u64]),big(Bool.true,[1465456035u64,3832110007u64,2127590384u64,112546216u64])) == big(Bool.false,[874773119u64,930255927u64,2125635947u64,112546216u64])
expect big_floor(big(Bool.true,[590682916u64,2901854080u64,1954437u64]),big(Bool.true,[1465456035u64,3832110007u64,2127590384u64,112546216u64])) == big(Bool.false,[])
expect big_mul(big(Bool.true,[590682916u64,2901854080u64,1954437u64]),14176324786099767627u64) == big(Bool.true,[1917204364u64,464242815u64,1771280571u64,2413643466u64,1501985u64])
expect big_add(big(Bool.false,[2566317101u64,3228874277u64,10146603u64]),big(Bool.false,[999180092u64])) == big(Bool.false,[3565497193u64,3228874277u64,10146603u64])
expect big_sub(big(Bool.false,[2566317101u64,3228874277u64,10146603u64]),big(Bool.false,[999180092u64])) == big(Bool.false,[1567137009u64,3228874277u64,10146603u64])
expect big_floor(big(Bool.false,[2566317101u64,3228874277u64,10146603u64]),big(Bool.false,[999180092u64])) == big(Bool.false,[2757058543u64,43615091u64])
expect big_mul(big(Bool.false,[2566317101u64,3228874277u64,10146603u64]),9251171088489038094u64) == big(Bool.false,[1380345718u64,3702132558u64,1902146676u64,3016391725u64,5088592u64])
expect big_add(big(Bool.true,[3492198858u64,2438036554u64,466080884u64,2644972115u64,4046078285u64,100830u64]),big(Bool.true,[2234625287u64,169530u64])) == big(Bool.true,[1431856849u64,2438206085u64,466080884u64,2644972115u64,4046078285u64,100830u64])
expect big_sub(big(Bool.true,[3492198858u64,2438036554u64,466080884u64,2644972115u64,4046078285u64,100830u64]),big(Bool.true,[2234625287u64,169530u64])) == big(Bool.true,[1257573571u64,2437867024u64,466080884u64,2644972115u64,4046078285u64,100830u64])
expect big_floor(big(Bool.true,[3492198858u64,2438036554u64,466080884u64,2644972115u64,4046078285u64,100830u64]),big(Bool.true,[2234625287u64,169530u64])) == big(Bool.false,[2340025208u64,1121166217u64,1897727670u64,2554499318u64])
expect big_mul(big(Bool.true,[3492198858u64,2438036554u64,466080884u64,2644972115u64,4046078285u64,100830u64]),15653781505417117848u64) == big(Bool.true,[667504624u64,2716176904u64,684702963u64,1179900742u64,1124673985u64,4009923533u64,1938531413u64,85564u64])
expect count_matches(compile("ab+c"),Str.to_utf8("abbbbcabcabcc"),0,Bool.false,0u64) == 3
expect replace_regex(compile("ab+c"),Str.to_utf8("abbbbcabcabcc"),"X") == Str.to_utf8("XXXc")
expect count_matches(compile("(ab|cd)+"),Str.to_utf8("xxababcdzcd"),0,Bool.false,0u64) == 2
expect replace_regex(compile("(ab|cd)+"),Str.to_utf8("xxababcdzcd"),"X") == Str.to_utf8("xxXzX")
expect count_matches(compile("a[0-9]?b"),Str.to_utf8("a1b ab axb a11b"),0,Bool.false,0u64) == 2
expect replace_regex(compile("a[0-9]?b"),Str.to_utf8("a1b ab axb a11b"),"X") == Str.to_utf8("X X axb a11b")
expect count_matches(compile("^[a-z]+$"),Str.to_utf8("HELLO"),0,Bool.true,0u64) == 1
expect count_matches(compile("a.*b"),Str.to_utf8("a123b45b\na0b"),0,Bool.false,0u64) == 2
expect replace_regex(compile("a.*b"),Str.to_utf8("a123b45b\na0b"),"X") == Str.to_utf8("X\nX")
expect count_matches(compile("<[^>]*>"),Str.to_utf8("one<two><three>four"),0,Bool.false,0u64) == 2
expect replace_regex(compile("<[^>]*>"),Str.to_utf8("one<two><three>four"),"X") == Str.to_utf8("oneXXfour")
expect count_matches(compile("a?"),Str.to_utf8("baab"),0,Bool.false,0u64) == 5
expect replace_regex(compile("a?"),Str.to_utf8("baab"),"X") == Str.to_utf8("XbXXXbX")
expect count_matches(compile("agggtaaa|tttaccct"),Str.to_utf8("AGGGTAAA tttaccct"),0,Bool.true,0u64) == 2
main! : List Arg => Result {} _
main! = |_args| Stdout.line!("general helper tests")
