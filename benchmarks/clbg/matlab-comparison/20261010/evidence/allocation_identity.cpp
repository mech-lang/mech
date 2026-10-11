// Untimed, generic diagnostic for the identity of nested MATLAB cell arrays.
// No construction, benchmark execution, mutation, or printed result computation.
#include "mex.h"
#include <unordered_set>
struct Inventory {
  size_t cells=0, leaves=0;
  bool two_slots=true;
  std::unordered_set<const void*> headers, storage;
};
static void inspect(const mxArray* array, Inventory& state) {
  if (!array || !mxIsCell(array)) return;
  const size_t size=mxGetNumberOfElements(array);
  ++state.cells;
  state.headers.insert(array);
  state.storage.insert(mxGetData(array));
  state.two_slots &= size==2;
  bool leaf=true;
  for(size_t index=0;index<size;++index) {
    const mxArray* child=mxGetCell(array,index);
    if(child && mxIsCell(child)) { leaf=false; inspect(child,state); }
  }
  state.leaves+=leaf;
}
void mexFunction(int outputs,mxArray** result,int inputs,const mxArray** arguments) {
  if(inputs!=1 || outputs!=1 || !mxIsCell(arguments[0]))
    mexErrMsgIdAndTxt("audit:input","Supply one nested cell array and request one inventory.");
  Inventory state;
  inspect(arguments[0],state);
  result[0]=mxCreateDoubleMatrix(1,5,mxREAL);
  double* out=mxGetDoubles(result[0]);
  out[0]=state.cells; out[1]=state.headers.size(); out[2]=state.storage.size();
  out[3]=state.leaves; out[4]=state.two_slots;
}
