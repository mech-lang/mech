function drag2 = foo()

% Constants

mass        = 4.3;
wing_area   = 0.6340;
wing_span   = 2.8;
c_w         = wing_area/wing_span;
fuse_length = 1.3590;
fuse_diam   = 0.1;
mu_air      = 0.0000143;
drag_ff     = 1.5;
g           = 9.8;
ee          = 0.80;
pi          = 3.141592654;
static_pressure = 10;
outside_temp = 35 + 273.15;
specific_gas_constant = 287.05;
Wx_dot = 0;
q = 20;
gamma_dot = 0;
V = 20;
mu = 1;
gamma = 1;
psi = 1;

% Calculate some values

rho = static_pressure/(outside_temp*specific_gas_constant);
nu = mu_air/rho;

% Coefficient of lift
C_L = 2*mass/(wing_area*rho*V*cos(mu))*(gamma_dot + g*cos(gamma)/V - Wx_dot*sin(gamma)^2*sin(psi));

% Lift
lift = q*wing_area*C_L;

% Calculate drag

Re_w = c_w*V/nu;
Re_f = fuse_length*V/nu;
a = (3.46*log(Re_w)/log(10)-5.6);
b = (3.46*log(Re_f)/log(10)-5.6);
drag = drag_ff*q*(3*wing_area/(a^2) + 0.9*pi*fuse_diam*fuse_length/(b^2));
drag2 = drag + lift^2/(wing_span^2*pi*ee)/q;

end