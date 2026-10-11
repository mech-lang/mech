app [main!] {
    pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU.tar.br",
}
import pf.Stdout
import pf.Arg exposing [Arg]
argument : List Arg -> U64
argument = |args|
    arg = (List.get(args, 1) |> Result.map_ok(Arg.display)) ?? "0"
    Str.to_u64(arg) ?? 0
get = |items, index|
    when List.get(items,index) is
        Ok(value) -> value
        Err(_) -> crash("index out of bounds")
fixed : F64, U64 -> Str
fixed = |value, places|
    factor = Num.pow_int(10u64, places)
    scaled : U64
    scaled = Num.round(Num.abs(value) * Num.to_f64(factor))
    whole = scaled // factor
    digits = Num.to_str(scaled % factor)
    padding = Str.repeat("0", places - Str.count_utf8_bytes(digits))
    sign = if value < 0 then "-" else ""
    "${sign}${Num.to_str(whole)}.${padding}${digits}"
Body : { x : F64, y : F64, z : F64, vx : F64, vy : F64, vz : F64, mass : F64 }
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
    Stdout.write!("${fixed(first,9)}\n${fixed(last,9)}\n")
