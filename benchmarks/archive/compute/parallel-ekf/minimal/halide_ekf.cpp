#include <Halide.h>
#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <string>
#include <vector>

using namespace Halide;
using A=std::array<Expr,9>;
static A mm(const A&a,const A&b){return {a[0]*b[0]+a[1]*b[3]+a[2]*b[6],a[0]*b[1]+a[1]*b[4]+a[2]*b[7],a[0]*b[2]+a[1]*b[5]+a[2]*b[8],a[3]*b[0]+a[4]*b[3]+a[5]*b[6],a[3]*b[1]+a[4]*b[4]+a[5]*b[7],a[3]*b[2]+a[4]*b[5]+a[5]*b[8],a[6]*b[0]+a[7]*b[3]+a[8]*b[6],a[6]*b[1]+a[7]*b[4]+a[8]*b[7],a[6]*b[2]+a[7]*b[5]+a[8]*b[8]};}
static A tr(const A&a){return {a[0],a[3],a[6],a[1],a[4],a[7],a[2],a[5],a[8]};}
int main(int ac,char**av){
 int N=ac>1?std::max(1,std::atoi(av[1])):10000,K=ac>2?std::max(1,std::atoi(av[2])):20;bool ck=ac>3&&std::string(av[3])=="checked";bool gpu=ac>4&&std::string(av[4])=="gpu";Var i("i");
 ImageParam x0(Float(32),1,"x0"),x1(Float(32),1,"x1"),x2(Float(32),1,"x2");
 ImageParam v(Float(32),1,"v"),w(Float(32),1,"w"),z(Float(32),1,"z");std::array<ImageParam,9> p={ImageParam(Float(32),1,"p0"),ImageParam(Float(32),1,"p1"),ImageParam(Float(32),1,"p2"),ImageParam(Float(32),1,"p3"),ImageParam(Float(32),1,"p4"),ImageParam(Float(32),1,"p5"),ImageParam(Float(32),1,"p6"),ImageParam(Float(32),1,"p7"),ImageParam(Float(32),1,"p8")};
 A P;for(int j=0;j<9;j++)P[j]=p[j](i);Expr th=x2(i),sn=sin(th),cs=cos(th),d=v(i)*0.1f,X=x0(i)+d*cs,Y=x1(i)+d*sn,Z=th+w(i)*0.1f;
 A F={1.f,0.f,-d*sn,0.f,1.f,d*cs,0.f,0.f,1.f};A U=mm(mm(F,P),tr(F));Expr j=cs*cs*.01f*.01f,k=cs*sn*.01f*.01f,l=sn*sn*.01f*.01f,h=.01f*.0025f;
 A Q={U[0]+j,U[1]+k,U[2],U[3]+k,U[4]+l,U[5],U[6],U[7],U[8]+h};Expr dx=140.f-X,dy=12.f-Y,rr=dx*dx+dy*dy,raw=z(i)-(atan2(dy,dx)-Z),nn=atan2(sin(raw),cos(raw)),h0=dy/rr,h1=-dx/rr,h2=-1.f;
 Expr q0=Q[0]*h0+Q[1]*h1+Q[2]*h2,q1=Q[3]*h0+Q[4]*h1+Q[5]*h2,q2=Q[6]*h0+Q[7]*h1+Q[8]*h2,iv=h0*q0+h1*q1+h2*q2+.25f,k0=q0/iv,k1=q1/iv,k2=q2/iv;
 A M={1.f-k0*h0,-k0*h1,-k0*h2,-k1*h0,1.f-k1*h1,-k1*h2,-k2*h0,-k2*h1,1.f-k2*h2};A C=mm(mm(M,Q),tr(M));A V={C[0]+k0*k0*.25f,C[1]+k0*k1*.25f,C[2]+k0*k2*.25f,C[3]+k1*k0*.25f,C[4]+k1*k1*.25f,C[5]+k1*k2*.25f,C[6]+k2*k0*.25f,C[7]+k2*k1*.25f,C[8]+k2*k2*.25f};Expr n0=X+k0*nn,n1=Y+k1*nn,n2=Z+k2*nn;
 Expr ok=abs(n0)<=3.402823466e38f&&abs(n1)<=3.402823466e38f&&abs(n2)<=3.402823466e38f;for(int j2=0;j2<9;j2++)ok=ok&&(abs(V[j2])<=3.402823466e38f);ok=ok&&(V[0]>0.f&&V[4]>0.f&&V[8]>0.f)&&abs(V[1]-V[3])<=.0001f&&abs(V[2]-V[6])<=.0001f&&abs(V[5]-V[7])<=.0001f;
 std::vector<Func> fs;for(int j2=0;j2<12;j2++)fs.emplace_back("o"+std::to_string(j2));fs[0](i)=ck?select(ok,n0,x0(i)):n0;fs[1](i)=ck?select(ok,n1,x1(i)):n1;fs[2](i)=ck?select(ok,n2,x2(i)):n2;for(int j2=0;j2<9;j2++)fs[j2+3](i)=ck?select(ok,V[j2],p[j2](i)):V[j2];if(gpu){Var block("block"),thread("thread");for(auto&f:fs)f.gpu_tile(i,block,thread,256,TailStrategy::GuardWithIf);}else for(auto&f:fs)f.parallel(i).vectorize(i,8);Pipeline pipe(fs);Target target=get_host_target();if(gpu)target=target.with_feature(Target::Metal);Callable callable;try{callable=pipe.compile_to_callable(pipe.infer_arguments(),target);}catch(const Halide::Error&e){std::cerr<<"compile_error: "<<e.what()<<"\n";return 2;}
 std::vector<Buffer<float>> a,b;for(int j2=0;j2<12;j2++){a.emplace_back(N);b.emplace_back(N);}Buffer<float> vb(N),wb(N),zb(N);for(int q3=0;q3<N;q3++){float ph=2.f*float(M_PI)*q3/N;vb(q3)=1.f+.05f*std::sin(3.f*ph);wb(q3)=.015f*(1.f+.1f*std::sin(2.f*ph));zb(q3)=-.55f+.01f*std::sin(7.f*ph)+.005f*std::sin(11.f*ph);a[0](q3)=55;a[1](q3)=25;a[2](q3)=.4f;for(int j2=0;j2<9;j2++)a[j2+3](q3)=j2==0||j2==4?100.f:j2==8?.15f:0.f;}
 auto turn=[&](){try{callable(a[3].raw_buffer(),a[4].raw_buffer(),a[5].raw_buffer(),a[6].raw_buffer(),a[7].raw_buffer(),a[8].raw_buffer(),a[9].raw_buffer(),a[10].raw_buffer(),a[11].raw_buffer(),vb.raw_buffer(),wb.raw_buffer(),a[0].raw_buffer(),a[1].raw_buffer(),a[2].raw_buffer(),zb.raw_buffer(),b[0].raw_buffer(),b[1].raw_buffer(),b[2].raw_buffer(),b[3].raw_buffer(),b[4].raw_buffer(),b[5].raw_buffer(),b[6].raw_buffer(),b[7].raw_buffer(),b[8].raw_buffer(),b[9].raw_buffer(),b[10].raw_buffer(),b[11].raw_buffer());}catch(const Halide::Error&e){std::cerr<<"runtime_error: "<<e.what()<<"\n";std::exit(2);}std::swap(a,b);};for(int q3=0;q3<5;q3++)for(int q4=0;q4<K;q4++)turn();if(gpu){for(auto&q:a)q.copy_to_host();for(auto&q:b)q.copy_to_host();}a[0].fill(55);a[1].fill(25);a[2].fill(.4f);for(int q3=0;q3<N;q3++)for(int j2=0;j2<9;j2++)a[j2+3](q3)=j2==0||j2==4?100.f:j2==8?.15f:0.f;auto st=std::chrono::steady_clock::now();for(int q3=0;q3<K;q3++)turn();double e=std::chrono::duration<double>(std::chrono::steady_clock::now()-st).count();if(gpu)for(auto&q:a)q.copy_to_host();double sum=0;for(auto&q:a)for(int q3=0;q3<N;q3++)sum+=q(q3);std::cout<<std::fixed<<std::setprecision(9)<<"lane: Halide "<<(gpu?"GPU Metal ":"")<<(ck?"checked":"unchecked")<<"\ninstances: "<<N<<"\nturns: "<<K<<"\nelapsed_s: "<<e<<"\nthroughput: "<<N*K/e<<"\nchecksum: "<<sum<<"\nvalidation: "<<(ck?"checked":"unchecked")<<"\nbackend: "<<(gpu?"metal":"cpu")<<"\n";
}
