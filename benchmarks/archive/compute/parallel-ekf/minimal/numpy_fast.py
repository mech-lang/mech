#!/usr/bin/env python3
import sys,time
import numpy as n

D=n.float32(.1);R=n.float32(.25);T=n.float32(.0001);Q0=n.float32(.01);Q1=n.float32(.0025)

def m(a,b):
    a0,a1,a2,a3,a4,a5,a6,a7,a8=a;b0,b1,b2,b3,b4,b5,b6,b7,b8=b
    return (a0*b0+a1*b3+a2*b6,a0*b1+a1*b4+a2*b7,a0*b2+a1*b5+a2*b8,a3*b0+a4*b3+a5*b6,a3*b1+a4*b4+a5*b7,a3*b2+a4*b5+a5*b8,a6*b0+a7*b3+a8*b6,a6*b1+a7*b4+a8*b7,a6*b2+a7*b5+a8*b8)
def t(a): return a[0],a[3],a[6],a[1],a[4],a[7],a[2],a[5],a[8]

def s(x0,x1,x2,p,v,w,b,ck):
    q=n.sin(x2);r=n.cos(x2);d=v*D;y0=x0+d*r;y1=x1+d*q;y2=x2+w*D
    f=(n.ones_like(x0),n.zeros_like(x0),-d*q,n.zeros_like(x0),n.ones_like(x0),d*r,n.zeros_like(x0),n.zeros_like(x0),n.ones_like(x0));u=m(m(f,p),t(f));j=r*r*D*D*Q0;k=r*q*D*D*Q0;l=q*q*D*D*Q0;h=D*D*Q1
    u=(u[0]+j,u[1]+k,u[2],u[3]+k,u[4]+l,u[5],u[6],u[7],u[8]+h);e0=n.float32(140)-y0;e1=n.float32(12)-y1;g=e0*e0+e1*e1;z=n.arctan2(e1,e0)-y2;a=b-z;z=n.arctan2(n.sin(a),n.cos(a));h0=e1/g;h1=-e0/g;h2=-1
    j=u[0]*h0+u[1]*h1+u[2]*h2;k=u[3]*h0+u[4]*h1+u[5]*h2;l=u[6]*h0+u[7]*h1+u[8]*h2;g=h0*j+h1*k+h2*l+R;j/=g;k/=g;l/=g
    a=(n.float32(1)-j*h0,-j*h1,-j*h2,-k*h0,n.float32(1)-k*h1,-k*h2,-l*h0,-l*h1,n.float32(1)-l*h2);c=m(m(a,u),t(a));u=(c[0]+j*j*R,c[1]+j*k*R,c[2]+j*l*R,c[3]+k*j*R,c[4]+k*k*R,c[5]+k*l*R,c[6]+l*j*R,c[7]+l*k*R,c[8]+l*l*R);y0+=j*z;y1+=k*z;y2+=l*z
    if ck:
        q=n.isfinite(y0)&n.isfinite(y1)&n.isfinite(y2);q &= n.logical_and.reduce([n.isfinite(x) for x in u]);q &= (u[0]>0)&(u[4]>0)&(u[8]>0);q &= n.abs(u[1]-u[3])<=T;q &= n.abs(u[2]-u[6])<=T;q &= n.abs(u[5]-u[7])<=T;f=int(n.count_nonzero(~q));n.copyto(x0,y0,where=q);n.copyto(x1,y1,where=q);n.copyto(x2,y2,where=q)
        for a,c in zip(p,u): n.copyto(a,c,where=q)
        return f
    x0[...]=y0;x1[...]=y1;x2[...]=y2
    for a,c in zip(p,u): a[...]=c
    return 0

def r(x0,x1,x2,p):
    x0.fill(n.float32(55));x1.fill(n.float32(25));x2.fill(n.float32(.4))
    for i,a in enumerate(p): a.fill(n.float32(100) if i in (0,4) else n.float32(.15) if i==8 else n.float32(0))

def main():
    N=max(1,int(sys.argv[1])) if len(sys.argv)>1 else 10000;K=max(1,int(sys.argv[2])) if len(sys.argv)>2 else 20;ck=len(sys.argv)>3 and sys.argv[3].lower()=='checked';i=n.arange(N,dtype=n.float32);a=n.float32(2*n.pi)*i/n.float32(N);v=1+n.float32(.05)*n.sin(3*a);w=n.float32(.015)*(1+n.float32(.1)*n.sin(2*a));b=n.float32(-.55)+n.float32(.01)*n.sin(7*a)+n.float32(.005)*n.sin(11*a);x0=n.full(N,n.float32(55));x1=n.full(N,n.float32(25));x2=n.full(N,n.float32(.4));p=[n.full(N,n.float32(100) if i in (0,4) else n.float32(.15) if i==8 else n.float32(0)) for i in range(9)];faults=0
    for _ in range(5):
        for _ in range(K): faults+=s(x0,x1,x2,p,v,w,b,ck)
    r(x0,x1,x2,p);q=time.perf_counter()
    for _ in range(K): faults+=s(x0,x1,x2,p,v,w,b,ck)
    e=time.perf_counter()-q;print('lane: NumPy vectorized minimal');print(f'instances: {N}');print(f'turns: {K}');print(f'elapsed_s: {e:.9f}');print(f'throughput: {N*K/e:.3f}');print(f'checksum: {float(x0.astype(n.float64).sum()+x1.astype(n.float64).sum()+x2.astype(n.float64).sum()+sum(a.astype(n.float64).sum() for a in p)):.9f}');print(f'validation: {"checked" if ck else "unchecked"}');print(f'faults: {faults}')
if __name__=='__main__':main()
