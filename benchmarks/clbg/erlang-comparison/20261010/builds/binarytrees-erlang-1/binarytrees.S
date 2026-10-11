{module, binarytrees}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 19}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"binarytrees.erl",12}]}.
    {func_info,{atom,binarytrees},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,1},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,1},[{x,2}]}.
    {allocate,2,2}.
    {init_yregs,{list,[{y,0},{y,1}]}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"binarytrees.erl",13}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {test_heap,4,1}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{integer,6},{x,0},{x,0}}.
    {line,[{location,"binarytrees.erl",14}]}.
    {call_ext,1,{extfunc,lists,max,1}}.
    {line,[{location,"binarytrees.erl",16}]}.
    {gc_bif,'+',{f,0},1,[{x,0},{integer,1}],{y,1}}.
    {move,{x,0},{y,0}}.
    {move,{y,1},{x,0}}.
    {line,[{location,"binarytrees.erl",18}]}.
    {call,1,{f,10}}. % bottomUp/1
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
    {call,1,{f,13}}. % itemCheck/1
    {'%',{var_info,{x,0},[{type,{t_integer,{1,'+inf'}}}]}}.
    {test_heap,4,1}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{y,1},{x,0},{x,1}}.
    {init_yregs,{list,[{y,1}]}}.
    {move,{literal,"stretch tree of depth ~w\t check: ~w~n"},{x,0}}.
    {line,[{location,"binarytrees.erl",17}]}.
    {call_ext,2,{extfunc,io,fwrite,2}}.
    {move,{y,0},{x,0}}.
    {line,[{location,"binarytrees.erl",20}]}.
    {call,1,{f,10}}. % bottomUp/1
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
    {line,[{location,"binarytrees.erl",21}]}.
    {call,2,{f,4}}. % depthLoop/2
    {'%',{var_info,{x,0},[{type,{t_atom,[ok]}}]}}.
    {move,{y,1},{x,0}}.
    {move,{y,0},{y,1}}.
    {trim,1,1}.
    {line,[{location,"binarytrees.erl",24}]}.
    {call,1,{f,13}}. % itemCheck/1
    {'%',{var_info,{x,0},[{type,{t_integer,{1,'+inf'}}}]}}.
    {test_heap,4,1}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{y,0},{x,0},{x,1}}.
    {trim,1,0}.
    {move,{literal,"long lived tree of depth ~w\t check: ~w~n"},{x,0}}.
    {line,[{location,"binarytrees.erl",23}]}.
    {call_ext,2,{extfunc,io,fwrite,2}}.
    {move,{integer,0},{x,0}}.
    {line,[{location,"binarytrees.erl",26}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.


{function, depthLoop, 2, 4}.
  {label,3}.
    {line,[{location,"binarytrees.erl",29}]}.
    {func_info,{atom,binarytrees},{atom,depthLoop},2}.
  {label,4}.
    {'%',{var_info,{x,0},[{type,{t_integer,{4,'+inf'}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_number,any}}]}}.
    {test,is_lt,
          {f,5},
          [{tr,{x,1},{t_number,any}},{tr,{x,0},{t_integer,{4,'+inf'}}}]}.
    {move,{atom,ok},{x,0}}.
    return.
  {label,5}.
    {line,[{location,"binarytrees.erl",31}]}.
    {gc_bif,'-',
            {f,0},
            2,
            [{tr,{x,1},{t_number,{4,'+inf'}}},
             {tr,{x,0},{t_integer,{4,'+inf'}}}],
            {x,2}}.
    {gc_bif,'+',{f,0},3,[{tr,{x,2},{t_number,any}},{integer,4}],{x,2}}.
    {gc_bif,'bsl',{f,0},3,[{integer,1},{tr,{x,2},{t_number,any}}],{x,2}}.
    {allocate,3,3}.
    {move,{x,2},{y,0}}.
    {move,{x,1},{y,1}}.
    {move,{x,0},{y,2}}.
    {move,{x,0},{x,1}}.
    {move,{x,2},{x,0}}.
    {move,{integer,0},{x,2}}.
    {line,[{location,"binarytrees.erl",33}]}.
    {call,3,{f,7}}. % sumLoop/3
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {test_heap,6,1}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{y,2},{x,0},{x,0}}.
    {put_list,{y,0},{x,0},{x,1}}.
    {trim,1,2}.
    {move,{literal,"~w\t trees of depth ~w\t check: ~w~n"},{x,0}}.
    {line,[{location,"binarytrees.erl",32}]}.
    {call_ext,2,{extfunc,io,fwrite,2}}.
    {line,[{location,"binarytrees.erl",34}]}.
    {gc_bif,'+',{f,0},0,[{tr,{y,1},{t_integer,{4,'+inf'}}},{integer,2}],{x,0}}.
    {move,{y,0},{x,1}}.
    {call_last,2,{f,4},2}. % depthLoop/2


{function, sumLoop, 3, 7}.
  {label,6}.
    {line,[{location,"binarytrees.erl",36}]}.
    {func_info,{atom,binarytrees},{atom,sumLoop},3}.
  {label,7}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,{4,'+inf'}}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,{0,'+inf'}}}]}}.
    {test,is_eq_exact,{f,8},[{tr,{x,0},{t_integer,any}},{integer,0}]}.
    {move,{x,2},{x,0}}.
    return.
  {label,8}.
    {line,[{location,"binarytrees.erl",38}]}.
    {gc_bif,'-',{f,0},3,[{tr,{x,0},{t_integer,any}},{integer,1}],{x,0}}.
    {allocate,3,3}.
    {move,{x,0},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,1},{y,2}}.
    {move,{x,1},{x,0}}.
    {call,1,{f,10}}. % bottomUp/1
    {'%',
     {var_info,
      {x,0},
      [{type,
        {t_tuple,2,true,
         #{1 =>
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
           2 =>
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
                 none,none}}}}}}]}}.
    {call,1,{f,13}}. % itemCheck/1
    {'%',{var_info,{x,0},[{type,{t_integer,{1,'+inf'}}}]}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,1},{t_integer,{0,'+inf'}}},
             {tr,{x,0},{t_integer,{1,'+inf'}}}],
            {x,2}}.
    {move,{y,2},{x,1}}.
    {move,{y,0},{x,0}}.
    {call_last,3,{f,7},3}. % sumLoop/3


{function, bottomUp, 1, 10}.
  {label,9}.
    {line,[{location,"binarytrees.erl",40}]}.
    {func_info,{atom,binarytrees},{atom,bottomUp},1}.
  {label,10}.
    {'%',{var_info,{x,0},[{type,{t_number,any}}]}}.
    {test,is_eq_exact,{f,11},[{tr,{x,0},{t_number,any}},{integer,0}]}.
    {move,{literal,{nil,nil}},{x,0}}.
    return.
  {label,11}.
    {line,[{location,"binarytrees.erl",41}]}.
    {gc_bif,'-',{f,0},1,[{tr,{x,0},{t_number,any}},{integer,1}],{x,1}}.
    {allocate,1,2}.
    {move,{x,0},{y,0}}.
    {move,{x,1},{x,0}}.
    {call,1,{f,10}}. % bottomUp/1
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
    {call,1,{f,10}}. % bottomUp/1
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


{function, itemCheck, 1, 13}.
  {label,12}.
    {line,[{location,"binarytrees.erl",43}]}.
    {func_info,{atom,binarytrees},{atom,itemCheck},1}.
  {label,13}.
    {test,is_tuple,{f,14},[{x,0}]}.
    {test,test_arity,{f,12},[{x,0},2]}.
    {allocate,2,1}.
    {init_yregs,{list,[{y,0}]}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{y,1}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"binarytrees.erl",45}]}.
    {call,1,{f,13}}. % itemCheck/1
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {gc_bif,'+',{f,0},1,[{tr,{x,0},{t_integer,{0,'+inf'}}},{integer,1}],{y,0}}.
    {move,{y,1},{x,0}}.
    {move,{y,0},{y,1}}.
    {trim,1,1}.
    {call,1,{f,13}}. % itemCheck/1
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,0},{t_integer,{1,'+inf'}}},
             {tr,{x,0},{t_integer,{0,'+inf'}}}],
            {x,0}}.
    {deallocate,1}.
    return.
  {label,14}.
    {test,is_eq_exact,{f,12},[{x,0},{atom,nil}]}.
    {move,{integer,0},{x,0}}.
    return.


{function, module_info, 0, 16}.
  {label,15}.
    {line,[]}.
    {func_info,{atom,binarytrees},{atom,module_info},0}.
  {label,16}.
    {move,{atom,binarytrees},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 18}.
  {label,17}.
    {line,[]}.
    {func_info,{atom,binarytrees},{atom,module_info},1}.
  {label,18}.
    {move,{x,0},{x,1}}.
    {move,{atom,binarytrees},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.
