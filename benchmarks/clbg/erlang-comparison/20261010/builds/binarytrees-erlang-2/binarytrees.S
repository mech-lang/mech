{module, binarytrees}.  %% version = 0

{exports, [{depth,2},{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 22}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"binarytrees.erl",13}]}.
    {func_info,{atom,binarytrees},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,1},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,1},[{x,2}]}.
    {allocate,2,2}.
    {init_yregs,{list,[{y,0},{y,1}]}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"binarytrees.erl",14}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {test_heap,4,1}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{integer,6},{x,0},{x,0}}.
    {line,[{location,"binarytrees.erl",15}]}.
    {call_ext,1,{extfunc,lists,max,1}}.
    {line,[{location,"binarytrees.erl",17}]}.
    {gc_bif,'+',{f,0},1,[{x,0},{integer,1}],{y,1}}.
    {move,{x,0},{y,0}}.
    {move,{y,1},{x,0}}.
    {line,[{location,"binarytrees.erl",19}]}.
    {call,1,{f,11}}. % bottomUp/1
    {'%',
     {var_info,
      {x,0},
      [{type,
        {t_tuple,2,true,
         #{1 =>
            {t_union,
             {t_atom,[nil]},
             none,none,
             {t_tuple,2,true,
              #{1 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none},
                2 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none}}},
             none,none},
           2 =>
            {t_union,
             {t_atom,[nil]},
             none,none,
             {t_tuple,2,true,
              #{1 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none},
                2 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none}}},
             none,none}}}}]}}.
    {call,1,{f,14}}. % itemCheck/1
    {'%',{var_info,{x,0},[{type,{t_integer,{1,'+inf'}}}]}}.
    {test_heap,4,1}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{y,1},{x,0},{x,1}}.
    {init_yregs,{list,[{y,1}]}}.
    {move,{literal,"stretch tree of depth ~w\t check: ~w~n"},{x,0}}.
    {line,[{location,"binarytrees.erl",18}]}.
    {call_ext,2,{extfunc,io,fwrite,2}}.
    {move,{y,0},{x,0}}.
    {line,[{location,"binarytrees.erl",21}]}.
    {call,1,{f,11}}. % bottomUp/1
    {'%',
     {var_info,
      {x,0},
      [{type,
        {t_tuple,2,true,
         #{1 =>
            {t_union,
             {t_atom,[nil]},
             none,none,
             {t_tuple,2,true,
              #{1 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none},
                2 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none}}},
             none,none},
           2 =>
            {t_union,
             {t_atom,[nil]},
             none,none,
             {t_tuple,2,true,
              #{1 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none},
                2 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none}}},
             none,none}}}}]}}.
    {move,{x,0},{y,1}}.
    {move,{y,0},{x,1}}.
    {move,{integer,4},{x,0}}.
    {line,[{location,"binarytrees.erl",22}]}.
    {call,2,{f,4}}. % depthLoop/2
    {move,{y,1},{x,0}}.
    {move,{y,0},{y,1}}.
    {trim,1,1}.
    {line,[{location,"binarytrees.erl",25}]}.
    {call,1,{f,14}}. % itemCheck/1
    {'%',{var_info,{x,0},[{type,{t_integer,{1,'+inf'}}}]}}.
    {test_heap,4,1}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{y,0},{x,0},{x,1}}.
    {trim,1,0}.
    {move,{literal,"long lived tree of depth ~w\t check: ~w~n"},{x,0}}.
    {line,[{location,"binarytrees.erl",24}]}.
    {call_ext,2,{extfunc,io,fwrite,2}}.
    {move,{integer,0},{x,0}}.
    {line,[{location,"binarytrees.erl",27}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.


{function, depthLoop, 2, 4}.
  {label,3}.
    {line,[{location,"binarytrees.erl",30}]}.
    {func_info,{atom,binarytrees},{atom,depthLoop},2}.
  {label,4}.
    {'%',{var_info,{x,0},[{type,{t_integer,{4,4}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_number,any}}]}}.
    {allocate,1,2}.
    {move,{x,1},{y,0}}.
    {move,{integer,2},{x,2}}.
    {line,[{location,"binarytrees.erl",31}]}.
    {call_ext,3,{extfunc,lists,seq,3}}.
    {test_heap,2,1}.
    {put_list,{y,0},nil,{x,1}}.
    {move,{x,0},{x,2}}.
    {trim,1,0}.
    {move,{literal,{binarytrees,depth}},{x,0}}.
    {call_ext,3,{extfunc,rpc,pmap,3}}.
    {test_heap,{alloc,[{words,0},{floats,0},{funs,1}]},1}.
    {move,{x,0},{x,1}}.
    {make_fun3,{f,21},0,0,{x,0},{list,[]}}.
    {line,[{location,"binarytrees.erl",32}]}.
    {call_ext_last,2,{extfunc,lists,foreach,2},0}.


{function, depth, 2, 6}.
  {label,5}.
    {line,[{location,"binarytrees.erl",37}]}.
    {func_info,{atom,binarytrees},{atom,depth},2}.
  {label,6}.
    {line,[{location,"binarytrees.erl",38}]}.
    {gc_bif,'-',{f,0},2,[{x,1},{x,0}],{x,1}}.
    {gc_bif,'+',{f,0},2,[{tr,{x,1},{t_number,any}},{integer,4}],{x,1}}.
    {gc_bif,'bsl',{f,0},2,[{integer,1},{tr,{x,1},{t_number,any}}],{x,1}}.
    {allocate,2,2}.
    {move,{x,1},{y,0}}.
    {move,{x,0},{y,1}}.
    {move,{integer,0},{x,2}}.
    {swap,{x,0},{x,1}}.
    {line,[{location,"binarytrees.erl",39}]}.
    {call,3,{f,8}}. % sumLoop/3
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {test_heap,6,1}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{y,1},{x,0},{x,0}}.
    {put_list,{y,0},{x,0},{x,0}}.
    {deallocate,2}.
    return.


{function, sumLoop, 3, 8}.
  {label,7}.
    {line,[{location,"binarytrees.erl",41}]}.
    {func_info,{atom,binarytrees},{atom,sumLoop},3}.
  {label,8}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_number,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,{0,'+inf'}}}]}}.
    {test,is_eq_exact,{f,9},[{tr,{x,0},{t_integer,any}},{integer,0}]}.
    {move,{x,2},{x,0}}.
    return.
  {label,9}.
    {line,[{location,"binarytrees.erl",43}]}.
    {gc_bif,'-',{f,0},3,[{tr,{x,0},{t_integer,any}},{integer,1}],{x,0}}.
    {allocate,3,3}.
    {move,{x,0},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,1},{y,2}}.
    {move,{x,1},{x,0}}.
    {call,1,{f,11}}. % bottomUp/1
    {'%',
     {var_info,
      {x,0},
      [{type,
        {t_tuple,2,true,
         #{1 =>
            {t_union,
             {t_atom,[nil]},
             none,none,
             {t_tuple,2,true,
              #{1 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none},
                2 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none}}},
             none,none},
           2 =>
            {t_union,
             {t_atom,[nil]},
             none,none,
             {t_tuple,2,true,
              #{1 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none},
                2 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none}}},
             none,none}}}}]}}.
    {call,1,{f,14}}. % itemCheck/1
    {'%',{var_info,{x,0},[{type,{t_integer,{1,'+inf'}}}]}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,1},{t_integer,{0,'+inf'}}},
             {tr,{x,0},{t_integer,{1,'+inf'}}}],
            {x,2}}.
    {move,{y,2},{x,1}}.
    {move,{y,0},{x,0}}.
    {call_last,3,{f,8},3}. % sumLoop/3


{function, bottomUp, 1, 11}.
  {label,10}.
    {line,[{location,"binarytrees.erl",45}]}.
    {func_info,{atom,binarytrees},{atom,bottomUp},1}.
  {label,11}.
    {'%',{var_info,{x,0},[{type,{t_number,any}}]}}.
    {test,is_eq_exact,{f,12},[{tr,{x,0},{t_number,any}},{integer,0}]}.
    {move,{literal,{nil,nil}},{x,0}}.
    return.
  {label,12}.
    {line,[{location,"binarytrees.erl",46}]}.
    {gc_bif,'-',{f,0},1,[{tr,{x,0},{t_number,any}},{integer,1}],{x,1}}.
    {allocate,1,2}.
    {move,{x,0},{y,0}}.
    {move,{x,1},{x,0}}.
    {call,1,{f,11}}. % bottomUp/1
    {'%',
     {var_info,
      {x,0},
      [{type,
        {t_tuple,2,true,
         #{1 =>
            {t_union,
             {t_atom,[nil]},
             none,none,
             {t_tuple,2,true,
              #{1 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none},
                2 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none}}},
             none,none},
           2 =>
            {t_union,
             {t_atom,[nil]},
             none,none,
             {t_tuple,2,true,
              #{1 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none},
                2 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none}}},
             none,none}}}}]}}.
    {gc_bif,'-',{f,0},1,[{tr,{y,0},{t_number,any}},{integer,1}],{x,1}}.
    {move,{x,0},{y,0}}.
    {move,{x,1},{x,0}}.
    {call,1,{f,11}}. % bottomUp/1
    {'%',
     {var_info,
      {x,0},
      [{type,
        {t_tuple,2,true,
         #{1 =>
            {t_union,
             {t_atom,[nil]},
             none,none,
             {t_tuple,2,true,
              #{1 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none},
                2 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none}}},
             none,none},
           2 =>
            {t_union,
             {t_atom,[nil]},
             none,none,
             {t_tuple,2,true,
              #{1 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none},
                2 =>
                 {t_union,
                  {t_atom,[nil]},
                  none,none,
                  {t_tuple,2,true,
                   #{1 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none},
                     2 =>
                      {t_union,
                       {t_atom,[nil]},
                       none,none,
                       {t_tuple,2,true,
                        #{1 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none},
                          2 =>
                           {t_union,
                            {t_atom,[nil]},
                            none,none,
                            {t_tuple,0,false,#{}},
                            none,none}}},
                       none,none}}},
                  none,none}}},
             none,none}}}}]}}.
    {test_heap,3,1}.
    {put_tuple2,{x,0},{list,[{y,0},{x,0}]}}.
    {deallocate,1}.
    return.


{function, itemCheck, 1, 14}.
  {label,13}.
    {line,[{location,"binarytrees.erl",48}]}.
    {func_info,{atom,binarytrees},{atom,itemCheck},1}.
  {label,14}.
    {test,is_tuple,{f,15},[{x,0}]}.
    {test,test_arity,{f,13},[{x,0},2]}.
    {allocate,2,1}.
    {init_yregs,{list,[{y,0}]}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{y,1}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"binarytrees.erl",50}]}.
    {call,1,{f,14}}. % itemCheck/1
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {gc_bif,'+',{f,0},1,[{tr,{x,0},{t_integer,{0,'+inf'}}},{integer,1}],{y,0}}.
    {move,{y,1},{x,0}}.
    {move,{y,0},{y,1}}.
    {trim,1,1}.
    {call,1,{f,14}}. % itemCheck/1
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,0},{t_integer,{1,'+inf'}}},
             {tr,{x,0},{t_integer,{0,'+inf'}}}],
            {x,0}}.
    {deallocate,1}.
    return.
  {label,15}.
    {test,is_eq_exact,{f,13},[{x,0},{atom,nil}]}.
    {move,{integer,0},{x,0}}.
    return.


{function, module_info, 0, 17}.
  {label,16}.
    {line,[]}.
    {func_info,{atom,binarytrees},{atom,module_info},0}.
  {label,17}.
    {move,{atom,binarytrees},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 19}.
  {label,18}.
    {line,[]}.
    {func_info,{atom,binarytrees},{atom,module_info},1}.
  {label,19}.
    {move,{x,0},{x,1}}.
    {move,{atom,binarytrees},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-depthLoop/2-fun-0-', 1, 21}.
  {label,20}.
    {line,[{location,"binarytrees.erl",32}]}.
    {func_info,{atom,binarytrees},{atom,'-depthLoop/2-fun-0-'},1}.
  {label,21}.
    {move,{x,0},{x,1}}.
    {move,{literal,"~w\t trees of depth ~w\t check: ~w~n"},{x,0}}.
    {line,[{location,"binarytrees.erl",33}]}.
    {call_ext_only,2,{extfunc,io,fwrite,2}}.
