let m (a:[9]f32) (b:[9]f32):[9]f32 =
  [a[0]*b[0]+a[1]*b[3]+a[2]*b[6],a[0]*b[1]+a[1]*b[4]+a[2]*b[7],a[0]*b[2]+a[1]*b[5]+a[2]*b[8],a[3]*b[0]+a[4]*b[3]+a[5]*b[6],a[3]*b[1]+a[4]*b[4]+a[5]*b[7],a[3]*b[2]+a[4]*b[5]+a[5]*b[8],a[6]*b[0]+a[7]*b[3]+a[8]*b[6],a[6]*b[1]+a[7]*b[4]+a[8]*b[7],a[6]*b[2]+a[7]*b[5]+a[8]*b[8]]
let t (a:[9]f32):[9]f32 = [a[0],a[3],a[6],a[1],a[4],a[7],a[2],a[5],a[8]]
let f (x:f32):bool = f32.abs x <= 3.402823466e38f32
let s (x:[12]f32) (v:f32) (w:f32) (z:f32) (c:bool):[12]f32 =
  let th=x[2]
  let sn=f32.sin th
  let cs=f32.cos th
  let d=v*0.1f32
  let x0=x[0]+d*cs
  let x1=x[1]+d*sn
  let x2=th+w*0.1f32
  let p=[x[3],x[4],x[5],x[6],x[7],x[8],x[9],x[10],x[11]]
  let j=[1f32,0f32,-d*sn,0f32,1f32,d*cs,0f32,0f32,1f32]
  let q=m (m j p) (t j)
  let q=[q[0]+cs*cs*0.0001f32,q[1]+cs*sn*0.0001f32,q[2],q[3]+cs*sn*0.0001f32,q[4]+sn*sn*0.0001f32,q[5],q[6],q[7],q[8]+0.000025f32]
  let dx=140f32-x0
  let dy=12f32-x1
  let rr=dx*dx+dy*dy
  let raw=z-(f32.atan2 dy dx-x2)
  let inn=f32.atan2 (f32.sin raw) (f32.cos raw)
  let h0=dy/rr
  let h1 = -dx/rr
  let h2 = -1f32
  let q0=q[0]*h0+q[1]*h1+q[2]*h2
  let q1=q[3]*h0+q[4]*h1+q[5]*h2
  let q2=q[6]*h0+q[7]*h1+q[8]*h2
  let iv=h0*q0+h1*q1+h2*q2+0.25f32
  let k0=q0/iv
  let k1=q1/iv
  let k2=q2/iv
  let a=[1f32-k0*h0,-k0*h1,-k0*h2,-k1*h0,1f32-k1*h1,-k1*h2,-k2*h0,-k2*h1,1f32-k2*h2]
  let y=m (m a q) (t a)
  let y=[y[0]+k0*k0*0.25f32,y[1]+k0*k1*0.25f32,y[2]+k0*k2*0.25f32,y[3]+k1*k0*0.25f32,y[4]+k1*k1*0.25f32,y[5]+k1*k2*0.25f32,y[6]+k2*k0*0.25f32,y[7]+k2*k1*0.25f32,y[8]+k2*k2*0.25f32]
  let nx=x0+k0*inn
  let ny=x1+k1*inn
  let nz=x2+k2*inn
  let ok=f nx && f ny && f nz && all f y && y[0]>0f32 && y[4]>0f32 && y[8]>0f32 && f32.abs(y[1]-y[3])<=0.0001f32 && f32.abs(y[2]-y[6])<=0.0001f32 && f32.abs(y[5]-y[7])<=0.0001f32
  let g (new:f32) (old:f32):f32 = if c && !ok then old else new
  in [g nx x[0],g ny x[1],g nz x[2],g y[0] x[3],g y[1] x[4],g y[2] x[5],g y[3] x[6],g y[4] x[7],g y[5] x[8],g y[6] x[9],g y[7] x[10],g y[8] x[11]]
let u [n] (x:[n][12]f32) (v:[n]f32) (w:[n]f32) (z:[n]f32) (c:bool):[n][12]f32 = map (\i -> s x[i] v[i] w[i] z[i] c) (iota n)
let main [n] (v:[n]f32) (w:[n]f32) (z:[n]f32) (turns:i32) (checked:bool):f64 =
  let x0=map (\_ -> [55f32,25f32,0.4f32,100f32,0f32,0f32,0f32,100f32,0f32,0f32,0f32,0.15f32]) (iota n)
  let x=loop x=x0 for _i < turns do u x v w z checked
  in reduce (+) 0f64 (map (\r -> reduce (+) 0f64 (map f64.f32 r)) x)

-- These fixed-mode entry points let an AOT compiler propagate the mode and
-- remove the validation predicate from the unchecked kernel. `main` remains
-- the reference entry point used for the source-shaped comparison.
let main_unchecked [n] (v:[n]f32) (w:[n]f32) (z:[n]f32) (turns:i32):f64 =
  let x0=map (\_ -> [55f32,25f32,0.4f32,100f32,0f32,0f32,0f32,100f32,0f32,0f32,0f32,0.15f32]) (iota n)
  let x=loop x=x0 for _i < turns do u x v w z false
  in reduce (+) 0f64 (map (\r -> reduce (+) 0f64 (map f64.f32 r)) x)

let main_checked [n] (v:[n]f32) (w:[n]f32) (z:[n]f32) (turns:i32):f64 =
  let x0=map (\_ -> [55f32,25f32,0.4f32,100f32,0f32,0f32,0f32,100f32,0f32,0f32,0f32,0.15f32]) (iota n)
  let x=loop x=x0 for _i < turns do u x v w z true
  in reduce (+) 0f64 (map (\r -> reduce (+) 0f64 (map f64.f32 r)) x)
