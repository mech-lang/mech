import "../stdlib/common"
def solar = 4f64*f64.pi*f64.pi
def days = 365.24f64
def bodies =
  [[0f64,0,0,0,0,0,solar],
   [4.84143144246472090,-1.16032004402742839,-0.103622044471123109,
    0.00166007664274403694*days,0.00769901118419740425*days,-0.0000690460016972063023*days,0.000954791938424326609*solar],
   [8.34336671824457987,4.12479856412430479,-0.403523417114321381,
    -0.00276742510726862411*days,0.00499852801234917238*days,0.0000230417297573763929*days,0.000285885980666130812*solar],
   [12.8943695621391310,-15.1111514016986312,-0.223307578892655734,
    0.00296460137564761618*days,0.00237847173959480950*days,-0.0000296589568540237556*days,0.0000436624404335156298*solar],
   [15.3796971148509165,-25.9193146099879641,0.179258772950371181,
    0.00268067772490389322*days,0.00162824170038242295*days,-0.0000951592254519715870*days,0.0000515138902046611451*solar]]
def energy (b:[5][7]f64):f64 =
  loop e = 0f64 for i < 5 do
    let speed = reduce (+) 0f64 (map (\k -> b[i,k+3]*b[i,k+3]) (iota 3))
    let e = e+0.5*b[i,6]*speed
    in loop e = e for j in i+1..<5 do
      let r2 = reduce (+) 0f64 (map (\k -> (b[i,k]-b[j,k])**2) (iota 3))
      in e-b[i,6]*b[j,6]/f64.sqrt r2
type body = (f64,f64,f64,f64,f64,f64,f64)
type system = (body,body,body,body,body)
def pack (b:[7]f64):body = (b[0],b[1],b[2],b[3],b[4],b[5],b[6])
def unpack (b:body):[7]f64 = [b.0,b.1,b.2,b.3,b.4,b.5,b.6]
def pair (a:body) (b:body):(body,body) =
  let dx = a.0-b.0
  let dy = a.1-b.1
  let dz = a.2-b.2
  let r2 = dx*dx+dy*dy+dz*dz
  let mag = 0.01/(r2*f64.sqrt r2)
  let a = (a.0,a.1,a.2,a.3-dx*b.6*mag,a.4-dy*b.6*mag,a.5-dz*b.6*mag,a.6)
  let b = (b.0,b.1,b.2,b.3+dx*a.6*mag,b.4+dy*a.6*mag,b.5+dz*a.6*mag,b.6)
  in (a,b)
def position (a:body):body =
  (a.0+0.01*a.3,a.1+0.01*a.4,a.2+0.01*a.5,a.3,a.4,a.5,a.6)
def advance (bs:system):system =
  let (a,b,c,d,e) = bs
  let (a,b) = pair a b
  let (a,c) = pair a c
  let (a,d) = pair a d
  let (a,e) = pair a e
  let (b,c) = pair b c
  let (b,d) = pair b d
  let (b,e) = pair b e
  let (c,d) = pair c d
  let (c,e) = pair c e
  let (d,e) = pair d e
  in (position a,position b,position c,position d,position e)
entry main (n:i64) (_input:[]u8):[]u8 =
  let initial = loop b = copy bodies for k < 3 do
    let p = reduce (+) 0f64 (map (\row -> row[k+3]*row[6]) bodies)
    let b[0,k+3] = -p/solar
    in b
  let e = energy initial
  let final = loop b = (pack initial[0],pack initial[1],pack initial[2],pack initial[3],pack initial[4]) for _i < n do advance b
  let (a,b,c,d,f) = final
  in fixed 9 e ++ "\n" ++ fixed 9 (energy [unpack a,unpack b,unpack c,unpack d,unpack f]) ++ "\n"
