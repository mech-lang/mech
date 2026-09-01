#!/usr/bin/env python3
import sys
import time
import numpy as n

D=n.float32(.1)
R=n.float32(.25)
Q=n.array([[.01,0],[0,.0025]],n.float32)
T=n.float32(.0001)

def step(x,p,v,w,z,ck):
    t=x[2]
    s=n.sin(t)
    c=n.cos(t)
    d=v*D
    y=n.array([x[0]+d*c,x[1]+d*s,x[2]+w*D],n.float32)
    f=n.array([[1,0,-d*s],[0,1,d*c],[0,0,1]],n.float32)
    g=n.array([[c*D,0],[s*D,0],[0,D]],n.float32)
    r=f@p@f.T+g@Q@g.T
    e=n.array([140,12],n.float32)-y[:2]
    b=e@e
    m=n.arctan2(e[1],e[0])-y[2]
    q=n.arctan2(n.sin(z-m),n.cos(z-m))
    h=n.array([e[1]/b,-e[0]/b,-1],n.float32)
    k=(r@h)/(h@r@h+R)
    a=n.eye(3,dtype=n.float32)-n.outer(k,h)
    u=a@r@a.T+n.outer(k,k)*R
    xx=y+k*q
    valid=n.isfinite(xx).all()
    valid=valid and n.isfinite(u).all()
    valid=valid and (n.diag(u)>0).all()
    valid=valid and n.max(n.abs(u-u.T))<=T
    if ck and not valid:
        return 1
    x[:]=xx
    p[:]=u
    return 0

def main():
    z=max(1,int(sys.argv[1])) if len(sys.argv)>1 else 10000
    t=max(1,int(sys.argv[2])) if len(sys.argv)>2 else 20
    ck=len(sys.argv)>3 and sys.argv[3].lower()=='checked'
    i=n.arange(z,dtype=n.float32)
    a=n.float32(2*n.pi)*i/n.float32(z)
    v=1+n.float32(.05)*n.sin(3*a)
    w=n.float32(.015)*(1+n.float32(.1)*n.sin(2*a))
    b=n.float32(-.55)+n.float32(.01)*n.sin(7*a)+n.float32(.005)*n.sin(11*a)
    x=n.tile(n.array([55,25,.4],n.float32),(z,1))
    p=n.tile(n.diag(n.array([100,100,.15],n.float32)),(z,1,1))
    f=0
    for _ in range(5):
        for j in range(z):
            f+=step(x[j],p[j],v[j],w[j],b[j],ck)
    x[:]=[55,25,.4]
    p[:]=n.diag(n.array([100,100,.15],n.float32))
    q=time.perf_counter()
    for _ in range(t):
        for j in range(z):
            f+=step(x[j],p[j],v[j],w[j],b[j],ck)
    e=time.perf_counter()-q
    print('lane: NumPy scalar minimal')
    print(f'instances: {z}')
    print(f'turns: {t}')
    print(f'elapsed_s: {e:.9f}')
    print(f'throughput: {z*t/e:.3f}')
    print(f'checksum: {float(x.astype(n.float64).sum()+p.astype(n.float64).sum()):.9f}')
    print(f'validation: {"checked" if ck else "unchecked"}')
    print(f'faults: {f}')

if __name__=='__main__':
    main()
