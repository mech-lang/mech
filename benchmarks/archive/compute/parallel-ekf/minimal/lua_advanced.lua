local sin=math.sin
local cos=math.cos
local abs=math.abs
local atan2=math.atan
local pi=math.pi
local instances=math.max(1,tonumber(arg[1]) or 10000)
local turns=math.max(1,tonumber(arg[2]) or 5)
local checked=string.lower(arg[3] or "unchecked")=="checked"
local dt=0.1
local dt2=dt*dt
local q0=0.01
local q1=0.0025
local measurement_noise=0.25
local symmetry_tolerance=0.0001
local finite_limit=3.402823466e38

local function array()
  local result={}
  for i=1,instances do
    result[i]=0.0
  end
  return result
end

local velocity=array()
local angular_velocity=array()
local bearing=array()
local x0=array()
local x1=array()
local x2=array()
local p00=array()
local p01=array()
local p02=array()
local p10=array()
local p11=array()
local p12=array()
local p20=array()
local p21=array()
local p22=array()

local function reset()
  for i=1,instances do
    x0[i]=55.0
    x1[i]=25.0
    x2[i]=0.4
    p00[i]=100.0
    p01[i]=0.0
    p02[i]=0.0
    p10[i]=0.0
    p11[i]=100.0
    p12[i]=0.0
    p20[i]=0.0
    p21[i]=0.0
    p22[i]=0.15
  end
end

for i=1,instances do
  local phase=2.0*pi*(i-1)/instances
  velocity[i]=1.0+0.05*sin(3.0*phase)
  angular_velocity[i]=0.015*(1.0+0.1*sin(2.0*phase))
  bearing[i]=-0.55+0.01*sin(7.0*phase)+0.005*sin(11.0*phase)
end

local function finite(value)
  return value==value and value<=finite_limit and value>=-finite_limit
end

local function step(i)
  local theta=x2[i]
  local st,ct=sin(theta),cos(theta)
  local distance=velocity[i]*dt
  local predicted_x0=x0[i]+distance*ct
  local predicted_x1=x1[i]+distance*st
  local predicted_x2=theta+angular_velocity[i]*dt
  local f02=-distance*st
  local f12=distance*ct
  local ap00=p00[i]+f02*p20[i]
  local ap01=p01[i]+f02*p21[i]
  local ap02=p02[i]+f02*p22[i]
  local ap10=p10[i]+f12*p20[i]
  local ap11=p11[i]+f12*p21[i]
  local ap12=p12[i]+f12*p22[i]
  local ap20=p20[i]
  local ap21=p21[i]
  local ap22=p22[i]
  local process00=ct*ct*dt2*q0
  local process01=ct*st*dt2*q0
  local process11=st*st*dt2*q0
  local process22=dt2*q1
  local predicted_p00=ap00+ap02*f02+process00
  local predicted_p01=ap01+ap02*f12+process01
  local predicted_p02=ap02
  local predicted_p10=ap10+ap12*f02+process01
  local predicted_p11=ap11+ap12*f12+process11
  local predicted_p12=ap12
  local predicted_p20=ap20+ap22*f02
  local predicted_p21=ap21+ap22*f12
  local predicted_p22=ap22+process22
  local dx=140.0-predicted_x0
  local dy=12.0-predicted_x1
  local squared_range=dx*dx+dy*dy
  local predicted_bearing=atan2(dy,dx)-predicted_x2
  local raw_innovation=bearing[i]-predicted_bearing
  local innovation=atan2(sin(raw_innovation),cos(raw_innovation))
  local h0=dy/squared_range
  local h1=-dx/squared_range
  local h2=-1.0
  local pht0=predicted_p00*h0+predicted_p01*h1+predicted_p02*h2
  local pht1=predicted_p10*h0+predicted_p11*h1+predicted_p12*h2
  local pht2=predicted_p20*h0+predicted_p21*h1+predicted_p22*h2
  local variance=h0*pht0+h1*pht1+h2*pht2+measurement_noise
  local k0=pht0/variance
  local k1=pht1/variance
  local k2=pht2/variance
  local candidate_x0=predicted_x0+k0*innovation
  local candidate_x1=predicted_x1+k1*innovation
  local candidate_x2=predicted_x2+k2*innovation
  local a00=1.0-k0*h0
  local a01=-k0*h1
  local a02=-k0*h2
  local a10=-k1*h0
  local a11=1.0-k1*h1
  local a12=-k1*h2
  local a20=-k2*h0
  local a21=-k2*h1
  local a22=1.0-k2*h2
  local b00=a00*predicted_p00+a01*predicted_p10+a02*predicted_p20
  local b01=a00*predicted_p01+a01*predicted_p11+a02*predicted_p21
  local b02=a00*predicted_p02+a01*predicted_p12+a02*predicted_p22
  local b10=a10*predicted_p00+a11*predicted_p10+a12*predicted_p20
  local b11=a10*predicted_p01+a11*predicted_p11+a12*predicted_p21
  local b12=a10*predicted_p02+a11*predicted_p12+a12*predicted_p22
  local b20=a20*predicted_p00+a21*predicted_p10+a22*predicted_p20
  local b21=a20*predicted_p01+a21*predicted_p11+a22*predicted_p21
  local b22=a20*predicted_p02+a21*predicted_p12+a22*predicted_p22
  local candidate_p00=b00*a00+b01*a01+b02*a02+k0*k0*measurement_noise
  local candidate_p01=b00*a10+b01*a11+b02*a12+k0*k1*measurement_noise
  local candidate_p02=b00*a20+b01*a21+b02*a22+k0*k2*measurement_noise
  local candidate_p10=b10*a00+b11*a01+b12*a02+k1*k0*measurement_noise
  local candidate_p11=b10*a10+b11*a11+b12*a12+k1*k1*measurement_noise
  local candidate_p12=b10*a20+b11*a21+b12*a22+k1*k2*measurement_noise
  local candidate_p20=b20*a00+b21*a01+b22*a02+k2*k0*measurement_noise
  local candidate_p21=b20*a10+b21*a11+b22*a12+k2*k1*measurement_noise
  local candidate_p22=b20*a20+b21*a21+b22*a22+k2*k2*measurement_noise
  if checked then
    local valid=finite(candidate_x0) and finite(candidate_x1) and finite(candidate_x2)
    valid=valid and finite(candidate_p00) and finite(candidate_p01) and finite(candidate_p02)
    valid=valid and finite(candidate_p10) and finite(candidate_p11) and finite(candidate_p12)
    valid=valid and finite(candidate_p20) and finite(candidate_p21) and finite(candidate_p22)
    valid=valid and candidate_p00>0.0 and candidate_p11>0.0 and candidate_p22>0.0
    valid=valid and abs(candidate_p01-candidate_p10)<=symmetry_tolerance
    valid=valid and abs(candidate_p02-candidate_p20)<=symmetry_tolerance
    valid=valid and abs(candidate_p12-candidate_p21)<=symmetry_tolerance
    if not valid then
      return 1
    end
  end
  x0[i]=candidate_x0
  x1[i]=candidate_x1
  x2[i]=candidate_x2
  p00[i]=candidate_p00
  p01[i]=candidate_p01
  p02[i]=candidate_p02
  p10[i]=candidate_p10
  p11[i]=candidate_p11
  p12[i]=candidate_p12
  p20[i]=candidate_p20
  p21[i]=candidate_p21
  p22[i]=candidate_p22
  return 0
end

local function dispatch(count)
  local faults=0
  for _=1,count do
    for i=1,instances do
      faults=faults+step(i)
    end
  end
  return faults
end

reset()
dispatch(5)
reset()
local started=os.clock()
local faults=dispatch(turns)
local elapsed=os.clock()-started
local checksum=0.0
for i=1,instances do
  checksum=checksum+x0[i]+x1[i]+x2[i]
end
for i=1,instances do
  checksum=checksum+p00[i]+p01[i]+p02[i]+p10[i]+p11[i]+p12[i]+p20[i]+p21[i]+p22[i]
end
print("lane: Lua advanced fixed-shape flat")
print("instances: "..instances)
print("turns: "..turns)
print(string.format("elapsed_s: %.9f",elapsed))
print(string.format("throughput: %.3f",instances*turns/elapsed))
print(string.format("checksum: %.9f",checksum))
print("validation: "..(checked and "checked" or "unchecked"))
print("faults: "..faults)
