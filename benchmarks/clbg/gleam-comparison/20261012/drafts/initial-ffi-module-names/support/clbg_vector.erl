-module(clbg_vector).
-export([get/2, set/3]).
get(Values, Index) -> element(Index + 1, Values).
set(Values, Index, Value) -> setelement(Index + 1, Values, Value).
