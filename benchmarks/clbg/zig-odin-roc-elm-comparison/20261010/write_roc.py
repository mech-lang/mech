"""Roc alpha4 programs: every algorithm and general helper is Roc source."""
import json
import re
from bootstrap import D,sha
from write_elm import ALU

HEADER='''app [main!] {
    pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU.tar.br",
}
import pf.Stdout
import pf.Stdin
import pf.Arg exposing [Arg]
argument : List Arg -> U64
argument = |args|
    arg = (List.get(args, 1) |> Result.map_ok(Arg.display)) ?? "0"
    Str.to_u64(arg) ?? 0
get = |items, index|
    when List.get(items,index) is
        Ok(value) -> value
        Err(_) -> crash("index out of bounds")
'''

FORMAT='''fixed : F64, U64 -> Str
fixed = |value, places|
    factor = Num.pow_int(10u64, places)
    scaled : U64
    scaled = Num.round(Num.abs(value) * Num.to_f64(factor))
    whole = scaled // factor
    digits = Num.to_str(scaled % factor)
    padding = Str.repeat("0", places - Str.count_utf8_bytes(digits))
    sign = if value < 0 then "-" else ""
    "${sign}${Num.to_str(whole)}.${padding}${digits}"
'''

NBODY='''Body : { x : F64, y : F64, z : F64, vx : F64, vy : F64, vz : F64, mass : F64 }
solar : F64
solar = 4 * Num.pi * Num.pi
body : F64, F64, F64, F64, F64, F64, F64 -> Body
body = |x, y, z, vx, vy, vz, mass|
    {x,y,z,vx:vx*365.24,vy:vy*365.24,vz:vz*365.24,mass:mass*solar}
bodies = [
    body(0,0,0,0,0,0,1),
    body(4.84143144246472090,-1.16032004402742839,-0.103622044471123109,0.00166007664274403694,0.00769901118419740425,-0.0000690460016972063023,0.000954791938424326609),
    body(8.34336671824457987,4.12479856412430479,-0.403523417114321381,-0.00276742510726862411,0.00499852801234917238,0.0000230417297573763929,0.000285885980666130812),
    body(12.8943695621391310,-15.1111514016986312,-0.223307578892655734,0.00296460137564761618,0.00237847173959480950,-0.0000296589568540237556,0.0000436624404335156298),
    body(15.3796971148509165,-25.9193146099879641,0.179258772950371181,0.00268067772490389322,0.00162824170038242295,-0.0000951592254519715870,0.0000515138902046611451),
]
offset = |items|
    momentum = List.walk(items, {x:0f64,y:0f64,z:0f64}, |p,b| {x:p.x+b.vx*b.mass,y:p.y+b.vy*b.mass,z:p.z+b.vz*b.mass})
    sun = get(items,0)
    List.set(items,0,{sun & vx: -momentum.x/solar,vy: -momentum.y/solar,vz: -momentum.z/solar})
advance_pairs = |items, i, j|
    if i >= List.len(items) then
        List.map(items, |b| {b & x:b.x+0.01*b.vx,y:b.y+0.01*b.vy,z:b.z+0.01*b.vz})
    else if j >= List.len(items) then
        advance_pairs(items,i+1,i+2)
    else
        a = get(items,i)
        b = get(items,j)
        dx = a.x-b.x
        dy = a.y-b.y
        dz = a.z-b.z
        square = dx*dx+dy*dy+dz*dz
        magnitude = 0.01/(square*Num.sqrt(square))
        aa = {a & vx:a.vx-dx*b.mass*magnitude,vy:a.vy-dy*b.mass*magnitude,vz:a.vz-dz*b.mass*magnitude}
        bb = {b & vx:b.vx+dx*a.mass*magnitude,vy:b.vy+dy*a.mass*magnitude,vz:b.vz+dz*a.mass*magnitude}
        advance_pairs(List.set(List.set(items,i,aa),j,bb),i,j+1)
steps = |remaining, items|
    if remaining == 0 then items else steps(remaining-1,advance_pairs(items,0,1))
energy = |items|
    List.walk_with_index(items,0f64,|total,a,i|
        potential = List.walk(List.drop_first(items,i+1),0f64,|sum,b|
            dx = a.x-b.x
            dy = a.y-b.y
            dz = a.z-b.z
            sum-a.mass*b.mass/Num.sqrt(dx*dx+dy*dy+dz*dz),
        )
        total+0.5*a.mass*(a.vx*a.vx+a.vy*a.vy+a.vz*a.vz)+potential,
    )
main! : List Arg => Result {} _
main! = |args|
    initial = offset(bodies)
    first = energy(initial)
    last = energy(steps(argument(args),initial))
    Stdout.write!("${fixed(first,9)}\\n${fixed(last,9)}\\n")
'''

SPECTRAL='''coefficient : U64, U64 -> F64
coefficient = |i,j| 1 / Num.to_f64(((i+j)*(i+j+1)//2)+i+1)
multiply = |transposed, n, values|
    List.map(List.range({start:At(0u64),end:Before(n)}), |i|
        List.walk_with_index(values,0f64,|total,value,j|
            a = if transposed then coefficient(j,i) else coefficient(i,j)
            total+a*value,
        ),
    )
ata = |n, values| multiply(Bool.true,n,multiply(Bool.false,n,values))
power = |remaining, n, u, v|
    if remaining == 0 then (u,v) else
        v1 = ata(n,u)
        u1 = ata(n,v1)
        power(remaining-1,n,u1,v1)
main! : List Arg => Result {} _
main! = |args|
    n = argument(args)
    (u,v) = power(10,n,List.repeat(1f64,n),List.repeat(0f64,n))
    uv = List.sum(List.map2(u,v,Num.mul))
    vv = List.sum(List.map(v,|x| x*x))
    Stdout.write!("${fixed(Num.sqrt(uv/vv),9)}\\n")
'''

TREE='''Tree : [Node U64 Tree Tree, Empty]

build = |depth, identity|
    if depth == 0 then Node(identity,Empty,Empty) else
        Node(identity,build(depth-1,identity*2),build(depth-1,identity*2+1))
check = |tree|
    when tree is
        Empty -> 0u64
        Node(_,left,right) -> 1+check(left)+check(right)
temporary = |remaining, depth, total|
    if remaining == 0 then total else
        temporary(remaining-1,depth,total+check(build(depth,remaining)))
depths = |depth, maximum, output|
    if depth > maximum then output else
        iterations = Num.pow_int(2u64,maximum-depth+4)
        total = temporary(iterations,depth,0)
        line = "${Num.to_str(iterations)}\\t trees of depth ${Num.to_str(depth)}\\t check: ${Num.to_str(total)}\\n"
        depths(depth+2,maximum,Str.concat(output,line))
main! : List Arg => Result {} _
main! = |args|
    maximum = Num.max(6,argument(args))
    stretch = check(build(maximum+1,1))
    long_lived = build(maximum,1)
    rows = depths(4,maximum,"")
    retained = check(long_lived)
    Stdout.write!("stretch tree of depth ${Num.to_str(maximum+1)}\\t check: ${Num.to_str(stretch)}\\n${rows}long lived tree of depth ${Num.to_str(maximum)}\\t check: ${Num.to_str(retained)}\\n")
'''

FANN='''reverse_prefix = |items, first, last|
    if first >= last then items else reverse_prefix(List.swap(items,first,last),first+1,last-1)
flips = |items, total|
    first = get(items,0)
    if first == 0 then total else flips(reverse_prefix(items,0,first),total+1)
rotate = |items, first, i, rank|
    if i == rank then List.set(items,rank,first) else
        value = get(items,i+1)
        rotate(List.set(items,i,value),first,i+1,rank)
prepare = |rank, counts|
    if rank <= 1 then counts else prepare(rank-1,List.set(counts,rank-1,rank))
next = |n, rank, items, counts|
    if rank == n then Finished else
        first = get(items,0)
        rotated = rotate(items,first,0,rank)
        remaining = get(counts,rank)-1
        updated = List.set(counts,rank,remaining)
        if remaining > 0 then Next(rotated,updated,rank) else next(n,rank+1,rotated,updated)
enumerate = |n, items, counts, rank, index, checksum, maximum|
    prepared = prepare(rank,counts)
    current : I64
    current = flips(items,0)
    total = checksum+(if Num.is_even(index) then current else -current)
    largest = Num.max(maximum,current)
    when next(n,1,items,prepared) is
        Finished -> "${Num.to_str(total)}\\nPfannkuchen(${Num.to_str(n)}) = ${Num.to_str(largest)}\\n"
        Next(perm,cs,r) -> enumerate(n,perm,cs,r,index+1,total,largest)
main! : List Arg => Result {} _
main! = |args|
    n = argument(args)
    output = enumerate(n,List.range({start:At(0u64),end:Before(n)}),List.repeat(0u64,n),n,0u64,0i64,0i64)
    Stdout.write!(output)
'''

KNUC='''code = |byte|
    when byte is
        67 | 99 -> 1u64
        71 | 103 -> 2u64
        84 | 116 -> 3u64
        _ -> 0u64
encode = |text| List.walk(Str.to_utf8(text),0u64,|key,byte| key*4+code(byte))
decode = |width, key, bytes|
    if width == 0 then Str.from_utf8_lossy(List.reverse(bytes)) else
        char = get([65u8,67,71,84],key%4)
        decode(width-1,key//4,List.append(bytes,char))
counts = |width, bytes|
    mask = Num.pow_int(4u64,width)-1
    (_,result) = List.walk_with_index(bytes,(0u64,Dict.empty({})),|state,byte,index|
        (key,table) = state
        next_key = Num.bitwise_and(key*4+Num.to_u64(byte),mask)
        updated = if index+1 >= width then
            Dict.update(table,next_key,|previous| Ok((previous ?? 0)+1))
        else table
        (next_key,updated),
    )
    result
frequencies = |width, bytes|
    entries = Dict.to_list(counts(width,bytes))
    sorted = List.sort_with(entries,|a,b|
        (ka,ca) = a
        (kb,cb) = b
        if ca == cb then Num.compare(ka,kb) else Num.compare(cb,ca),
    )
    denominator = Num.to_f64(List.len(bytes)-width+1)
    rows = List.map(sorted,|pair|
        (key,total) = pair
        "${decode(width,key,[])} ${fixed(100*Num.to_f64(total)/denominator,3)}\\n",
    )
    Str.concat(Str.join_with(rows,""),"\\n")
main! : List Arg => Result {} _
main! = |_args|
    raw = Stdin.read_to_end!({})?
    text = Str.from_utf8_lossy(raw)
    record = (Str.split_first(text,">THREE") |> Result.map_ok(|x| x.after)) ?? ""
    rest = (Str.split_first(record,"\\n") |> Result.map_ok(|x| x.after)) ?? ""
    sequence = (Str.split_first(rest,">") |> Result.map_ok(|x| x.before)) ?? rest
    bytes = List.map(List.keep_if(Str.to_utf8(sequence),|b| b != 10 && b != 13),code)
    packed = List.map(bytes,Num.to_u8)
    first = frequencies(1,packed)
    second = frequencies(2,packed)
    queries = [(3u64,"GGT"),(4,"GGTA"),(6,"GGTATT"),(12,"GGTATTTTAATT"),(18,"GGTATTTTAATTTATAGT")]
    rows = List.map(queries,|pair|
        (width,query) = pair
        table = counts(width,packed)
        number = Dict.get(table,encode(query)) ?? 0
        "${Num.to_str(number)}\\t${query}\\n",
    )
    Stdout.write!(Str.concat(Str.concat(first,second),Str.join_with(rows,"")))
'''

MANDEL='''inside : U64, F64, F64, F64, F64 -> U8
inside = |remaining, cr, ci, zr, zi|
    if zr*zr+zi*zi > 4 then 0 else
        if remaining == 0 then 1 else inside(remaining-1,cr,ci,zr*zr-zi*zi+cr,2*zr*zi+ci)
byte = |n, row, x, bit, packed|
    if bit == 8 then packed else
        value = if x+bit < n then inside(50,2*Num.to_f64(x+bit)/Num.to_f64(n)-1.5,2*Num.to_f64(row)/Num.to_f64(n)-1,0,0) else 0
        byte(n,row,x,bit+1,Num.shift_left_by(packed,1)+value)
row_bytes = |n, row, x, bytes|
    if x >= n then bytes else row_bytes(n,row,x+8,List.append(bytes,byte(n,row,x,0,0u8)))
rows! = |n, row|
    if row >= n then Ok({}) else
        Stdout.write_bytes!(row_bytes(n,row,0,List.with_capacity((n+7)//8)))?
        rows!(n,row+1)
main! : List Arg => Result {} _
main! = |args|
    n = argument(args)
    Stdout.write!("P4\\n${Num.to_str(n)} ${Num.to_str(n)}\\n")?
    rows!(n,0)
'''

FASTA='''alu = "ALU"
cumulative : List U8, List F64 -> List (U8,F64)
cumulative = |letters, probabilities|
    (_,pairs) = List.walk(List.map2(letters,probabilities,|a,b| (a,b)),(0f64,[]),|state,pair|
        (total,items) = state
        (letter,probability) = pair
        next = total+probability
        (next,List.append(items,(letter,next))),
    )
    pairs
choose = |value, pairs|
    when pairs is
        [(letter,threshold),.. as rest] -> if value < threshold then letter else choose(value,rest)
        [] -> 116u8
line = |remaining, offset, seed, repeated, distribution, bytes|
    if remaining == 0 then (bytes,offset,seed) else
        next_seed = if repeated then seed else (seed*3877+29573)%139968
        value : F64
        value = Num.to_f64(next_seed)/139968.0
        letter = if repeated then get(Str.to_utf8(alu),offset%Str.count_utf8_bytes(alu)) else choose(value,distribution)
        line(remaining-1,offset+1,next_seed,repeated,distribution,List.append(bytes,letter))
emit! = |remaining, offset, seed, repeated, distribution, buffer|
    if remaining == 0 then
        Stdout.write_bytes!(buffer)?
        Ok(seed)
    else
        take = Num.min(60,remaining)
        (bytes,next_offset,next_seed) = line(take,offset,seed,repeated,distribution,buffer)
        next = List.append(bytes,10u8)
        if List.len(next) >= 65536 then
            Stdout.write_bytes!(next)?
            emit!(remaining-take,next_offset,next_seed,repeated,distribution,List.with_capacity(65600))
        else emit!(remaining-take,next_offset,next_seed,repeated,distribution,next)
main! : List Arg => Result {} _
main! = |args|
    n = argument(args)
    iub = cumulative(Str.to_utf8("acgtBDHKMNRSVWY"),[0.27,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02])
    homo = cumulative(Str.to_utf8("acgt"),[0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008])
    Stdout.write!(">ONE Homo sapiens alu\\n")?
    _ = emit!(2*n,0,42,Bool.true,[],List.with_capacity(65600))?
    Stdout.write!(">TWO IUB ambiguity codes\\n")?
    seed = emit!(3*n,0,42,Bool.false,iub,List.with_capacity(65600))?
    Stdout.write!(">THREE Homo sapiens frequency\\n")?
    _ = emit!(5*n,0,seed,Bool.false,homo,List.with_capacity(65600))?
    Ok({})
'''.replace('ALU',ALU)

REV='''complement = |byte|
    when byte is
        65 | 97 -> 84u8
        84 | 116 | 85 | 117 -> 65u8
        67 | 99 -> 71u8
        71 | 103 -> 67u8
        77 | 109 -> 75u8
        82 | 114 -> 89u8
        87 | 119 -> 87u8
        83 | 115 -> 83u8
        89 | 121 -> 82u8
        75 | 107 -> 77u8
        86 | 118 -> 66u8
        72 | 104 -> 68u8
        68 | 100 -> 72u8
        66 | 98 -> 86u8
        _ -> 78u8
wrapped! = |bytes, offset, buffer|
    if offset >= List.len(bytes) then Stdout.write_bytes!(buffer) else
        take = Num.min(60,List.len(bytes)-offset)
        line_bytes = List.sublist(bytes,{start:offset,len:take})
        next = List.append(List.concat(buffer,line_bytes),10u8)
        if List.len(next) >= 65536 then
            Stdout.write_bytes!(next)?
            wrapped!(bytes,offset+take,List.with_capacity(65600))
        else wrapped!(bytes,offset+take,next)
record! = |record|
    end = List.find_first_index(record,|b| b == 10) ?? List.len(record)
    header = List.prepend(List.take_first(record,end+1),62u8)
    Stdout.write_bytes!(header)?
    bases = List.keep_if(List.drop_first(record,end+1),|b| b != 10 && b != 13)
    sequence = List.map(List.reverse(bases),complement)
    wrapped!(sequence,0,List.with_capacity(65600))
main! : List Arg => Result {} _
main! = |_args|
    raw = Stdin.read_to_end!({})?
    records = List.drop_first(List.split_on(raw,62u8),1)
    List.for_each_try!(records,record!)
'''

def main():
    rows=[]
    def emit(bench,body,formatting=False):
        common=HEADER
        if 'Stdin.' not in body: common=common.replace('import pf.Stdin\n','')
        if 'argument(' not in body: common=re.sub(r'argument :.*?(?=get =)', '', common,flags=re.S)
        if 'get(' not in body: common=common[:common.index('get =')]
        source=common+(FORMAT if formatting else '')+body
        identifier=bench+'-roc-idiomatic'
        path=D/'sources'/(identifier+'.roc')
        path.write_text(source)
        rows.append(dict(key=bench+':'+identifier,id=identifier,benchmark=bench,language='roc',source=str(path),source_sha256=sha(path),
            source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/'+bench+'.html',
            local_adaptation=True,adaptations=['Locally authored Roc alpha4 workload. All arithmetic, collection updates, algorithm sequencing, general regex/big-integer helpers and formatting are in measured Roc source and included in LOC. General basic-cli process I/O only.'],libraries=['basic-cli 0.20.0']))
    for bench,body in [('nbody',NBODY),('spectralnorm',SPECTRAL),('binarytrees',TREE),('fannkuchredux',FANN),('knucleotide',KNUC),('mandelbrot',MANDEL),('fasta',FASTA),('revcomp',REV)]:
        emit(bench,body,bench in ('nbody','spectralnorm','knucleotide'))
    try:
        from roc_support import BIG,PI,REGEX,REGEX_BENCH
        emit('pidigits',BIG+PI)
        emit('regexredux',REGEX+REGEX_BENCH)
    except ImportError: pass
    (D/'written-roc.json').write_text(json.dumps(dict(programs=rows),indent=2)+'\n')
    print('WROTE ROC',len(rows),flush=True)

if __name__=='__main__': main()
