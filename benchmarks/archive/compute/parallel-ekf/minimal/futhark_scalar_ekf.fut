-- Scalarized Futhark control for the same one-worker resident-loop boundary.
-- Matrix products are expanded into scalar bindings so the C backend can
-- keep the covariance in registers instead of materializing 9-element arrays.

let finite (x:f32):bool = f32.abs x <= 3.402823466e38f32

let step (x:[12]f32) (v:f32) (w:f32) (z:f32) (checked:bool):[12]f32 =
  let th = x[2]
  let sn = f32.sin th
  let cs = f32.cos th
  let d = v*0.1f32
  let ds = d*sn
  let dc = d*cs
  let nx = x[0]+dc
  let ny = x[1]+ds
  let nt = th+w*0.1f32
  let p00 = x[3]
  let p01 = x[4]
  let p02 = x[5]
  let p10 = x[6]
  let p11 = x[7]
  let p12 = x[8]
  let p20 = x[9]
  let p21 = x[10]
  let p22 = x[11]
  let r00 = p00-ds*p20
  let r01 = p01-ds*p21
  let r02 = p02-ds*p22
  let r10 = p10+dc*p20
  let r11 = p11+dc*p21
  let r12 = p12+dc*p22
  let q00 = r00-ds*r02+cs*cs*0.0001f32
  let q01 = r01+dc*r02+cs*sn*0.0001f32
  let q02 = r02
  let q10 = r10-ds*r12+cs*sn*0.0001f32
  let q11 = r11+dc*r12+sn*sn*0.0001f32
  let q12 = r12
  let q20 = p20-ds*p22
  let q21 = p21+dc*p22
  let q22 = p22+0.000025f32
  let dx = 140f32-nx
  let dy = 12f32-ny
  let rr = dx*dx+dy*dy
  let raw = z-(f32.atan2 dy dx-nt)
  let inn = f32.atan2 (f32.sin raw) (f32.cos raw)
  let h0 = dy/rr
  let h1 = -dx/rr
  let h2 = -1f32
  let ph0 = q00*h0+q01*h1+q02*h2
  let ph1 = q10*h0+q11*h1+q12*h2
  let ph2 = q20*h0+q21*h1+q22*h2
  let iv = h0*ph0+h1*ph1+h2*ph2+0.25f32
  let k0 = ph0/iv
  let k1 = ph1/iv
  let k2 = ph2/iv
  let a00 = 1f32-k0*h0
  let a01 = -k0*h1
  let a02 = -k0*h2
  let a10 = -k1*h0
  let a11 = 1f32-k1*h1
  let a12 = -k1*h2
  let a20 = -k2*h0
  let a21 = -k2*h1
  let a22 = 1f32-k2*h2
  let b00 = a00*q00+a01*q10+a02*q20
  let b01 = a00*q01+a01*q11+a02*q21
  let b02 = a00*q02+a01*q12+a02*q22
  let b10 = a10*q00+a11*q10+a12*q20
  let b11 = a10*q01+a11*q11+a12*q21
  let b12 = a10*q02+a11*q12+a12*q22
  let b20 = a20*q00+a21*q10+a22*q20
  let b21 = a20*q01+a21*q11+a22*q21
  let b22 = a20*q02+a21*q12+a22*q22
  let y00 = b00*a00+b01*a01+b02*a02+k0*k0*0.25f32
  let y01 = b00*a10+b01*a11+b02*a12+k0*k1*0.25f32
  let y02 = b00*a20+b01*a21+b02*a22+k0*k2*0.25f32
  let y10 = b10*a00+b11*a01+b12*a02+k1*k0*0.25f32
  let y11 = b10*a10+b11*a11+b12*a12+k1*k1*0.25f32
  let y12 = b10*a20+b11*a21+b12*a22+k1*k2*0.25f32
  let y20 = b20*a00+b21*a01+b22*a02+k2*k0*0.25f32
  let y21 = b20*a10+b21*a11+b22*a12+k2*k1*0.25f32
  let y22 = b20*a20+b21*a21+b22*a22+k2*k2*0.25f32
  let cx = nx+k0*inn
  let cy = ny+k1*inn
  let ct = nt+k2*inn
  let ok = finite cx && finite cy && finite ct && finite y00 && finite y01 && finite y02 && finite y10 && finite y11 && finite y12 && finite y20 && finite y21 && finite y22
    && y00>0f32 && y11>0f32 && y22>0f32
    && f32.abs(y01-y10)<=0.0001f32
    && f32.abs(y02-y20)<=0.0001f32
    && f32.abs(y12-y21)<=0.0001f32
  let g (new:f32) (old:f32):f32 = if checked && !ok then old else new
  in [g cx x[0],g cy x[1],g ct x[2],g y00 x[3],g y01 x[4],g y02 x[5],g y10 x[6],g y11 x[7],g y12 x[8],g y20 x[9],g y21 x[10],g y22 x[11]]

let update [n] (x:[n][12]f32) (v:[n]f32) (w:[n]f32) (z:[n]f32) (checked:bool):[n][12]f32 =
  map (\i -> step x[i] v[i] w[i] z[i] checked) (iota n)

let initial [n]:[n][12]f32 =
  map (\_ -> [55f32,25f32,0.4f32,100f32,0f32,0f32,0f32,100f32,0f32,0f32,0f32,0.15f32]) (iota n)

let checksum [n] (x:[n][12]f32):f64 =
  reduce (+) 0f64 (map (\r -> reduce (+) 0f64 (map f64.f32 r)) x)

let main [n] (v:[n]f32) (w:[n]f32) (z:[n]f32) (turns:i32) (checked:bool):f64 =
  let x = loop x=initial for _i < turns do update x v w z checked
  in checksum x

let main_unchecked [n] (v:[n]f32) (w:[n]f32) (z:[n]f32) (turns:i32):f64 =
  let x = loop x=initial for _i < turns do update x v w z false
  in checksum x

let main_checked [n] (v:[n]f32) (w:[n]f32) (z:[n]f32) (turns:i32):f64 =
  let x = loop x=initial for _i < turns do update x v w z true
  in checksum x
