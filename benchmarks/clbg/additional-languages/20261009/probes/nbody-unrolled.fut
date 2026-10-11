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
def advance (b:*[5][7]f64):*[5][7]f64 =
  let b = #[unroll] loop b = b for i < 5 do
    #[unroll] loop b = b for j in i+1..<5 do
      let delta = map (\k -> b[i,k]-b[j,k]) (iota 3)
      let r2 = reduce (+) 0f64 (map (\d -> d*d) delta)
      let mag = 0.01/(r2*f64.sqrt r2)
      in #[unroll] loop b = b for k < 3 do
        let vi = b[i,k+3]-delta[k]*b[j,6]*mag
        let vj = b[j,k+3]+delta[k]*b[i,6]*mag
        let b[i,k+3] = vi
        let b[j,k+3] = vj
        in b
  in #[unroll] loop b = b for i < 5 do
    #[unroll] loop b = b for k < 3 do
      let b[i,k] = b[i,k]+0.01*b[i,k+3]
      in b
entry main (n:i64) (_input:[]u8):[]u8 =
  let initial = loop b = copy bodies for k < 3 do
    let p = reduce (+) 0f64 (map (\row -> row[k+3]*row[6]) bodies)
    let b[0,k+3] = -p/solar
    in b
  let e = energy initial
  let final = loop b = copy initial for _i < n do #[inline] advance b
  in fixed 9 e ++ "\n" ++ fixed 9 (energy final) ++ "\n"
