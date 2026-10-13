# C++ explicit construction operand ownership

This additive grammar repair addresses the existing type-shadow audit, separately
from constructor initializer counting. No benchmark source, producer count,
timing record or frozen audit is changed.

Immediately adjacent unqualified `new Type(...)` is an explicit construction
operand. It cannot declare an ordinary function named `Type`, so it must not
create a function-name shadow merely because a preceding return, call or binding
occurs in the same statement. The normal type environment still records actual
written local, parameter, function and member value declarations and their scope.
Placement, qualified and dependent construction forms are not newly certified.

The prior exact Node source review identified two later `new Node` call sites at
masked source offsets518 and539, outside constructor headers274 and299. The
working fix proves the call grammar itself; it does not remove diagnostics from
constructor-adjacent ranges or erase unknown same-name declarations.

One new type-environment fixture covers return, binding, call and ternary
contexts. One integration fixture retains the explicit member initializers and
subsequent return construction as separate written source work. These fixtures
have not run; admission requires a new frozen pass under the timing mutex.
