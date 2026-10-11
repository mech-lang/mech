{module, pidigits}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 28}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"pidigits.erl",12}]}.
    {func_info,{atom,pidigits},{atom,main},1}.
  {label,2}.
    {test,is_integer,{f,3},[{x,0}]}.
    {allocate_heap,1,{alloc,[{words,0},{floats,0},{funs,1}]},1}.
    {move,{x,0},{y,0}}.
    {make_fun3,{f,27},0,0,{x,0},{list,[]}}.
    {line,[{location,"pidigits.erl",13}]}.
    {call_ext,1,{extfunc,erlang,spawn_link,1}}.
    {move,{x,0},{x,1}}.
    {move,{atom,io_worker},{x,0}}.
    {line,[{location,"pidigits.erl",14}]}.
    {call_ext,2,{extfunc,erlang,register,2}}.
    {move,{integer,0},{x,2}}.
    {move,{integer,1},{x,1}}.
    {move,{y,0},{x,3}}.
    {move,{literal,{1,0,1}},{x,0}}.
    {call_last,4,{f,13},1}. % stream/4
  {label,3}.
    {test,is_nonempty_list,{f,1},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,1},[{x,2}]}.
    {allocate,0,2}.
    {move,{x,1},{x,0}}.
    {line,[{location,"pidigits.erl",16}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {call_last,1,{f,2},0}. % main/1


{function, comp, 2, 5}.
  {label,4}.
    {line,[{location,"pidigits.erl",18}]}.
    {func_info,{atom,pidigits},{atom,comp},2}.
  {label,5}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,3,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{1,'+inf'}}}}}]}}.
    {'%',{var_info,{x,1},
                   [{type,{t_tuple,3,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,{6,'+inf'}},
                                     3 => {t_integer,{3,'+inf'}}}}}]}}.
    {get_tuple_element,{x,0},0,{x,2}}.
    {get_tuple_element,{x,1},0,{x,3}}.
    {gc_bif,'*',
            {f,0},
            4,
            [{tr,{x,2},{t_integer,{1,'+inf'}}},
             {tr,{x,3},{t_integer,{1,'+inf'}}}],
            {x,3}}.
    {get_tuple_element,{x,1},1,{x,4}}.
    {gc_bif,'*',
            {f,0},
            5,
            [{tr,{x,2},{t_integer,{1,'+inf'}}},
             {tr,{x,4},{t_integer,{6,'+inf'}}}],
            {x,2}}.
    {get_tuple_element,{x,1},2,{x,1}}.
    {get_tuple_element,{x,0},1,{x,4}}.
    {gc_bif,'*',
            {f,0},
            5,
            [{tr,{x,4},{t_integer,any}},{tr,{x,1},{t_integer,{3,'+inf'}}}],
            {x,4}}.
    {gc_bif,'+',
            {f,0},
            5,
            [{tr,{x,2},{t_integer,{6,'+inf'}}},{tr,{x,4},{t_integer,any}}],
            {x,2}}.
    {get_tuple_element,{x,0},2,{x,0}}.
    {gc_bif,'*',
            {f,0},
            4,
            [{tr,{x,0},{t_integer,{1,'+inf'}}},
             {tr,{x,1},{t_integer,{3,'+inf'}}}],
            {x,0}}.
    {test_heap,4,4}.
    {put_tuple2,{x,0},{list,[{x,3},{x,2},{x,0}]}}.
    return.


{function, next, 1, 7}.
  {label,6}.
    {line,[{location,"pidigits.erl",20}]}.
    {func_info,{atom,pidigits},{atom,next},1}.
  {label,7}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,3,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{1,'+inf'}}}}}]}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {gc_bif,'*',{f,0},2,[{tr,{x,1},{t_integer,{1,'+inf'}}},{integer,3}],{x,1}}.
    {get_tuple_element,{x,0},1,{x,2}}.
    {gc_bif,'+',
            {f,0},
            3,
            [{tr,{x,1},{t_integer,{3,'+inf'}}},{tr,{x,2},{t_integer,any}}],
            {x,1}}.
    {get_tuple_element,{x,0},2,{x,0}}.
    {gc_bif,'div',
            {f,0},
            2,
            [{tr,{x,1},{t_integer,any}},{tr,{x,0},{t_integer,{1,'+inf'}}}],
            {x,0}}.
    return.


{function, safe, 2, 9}.
  {label,8}.
    {line,[{location,"pidigits.erl",22}]}.
    {func_info,{atom,pidigits},{atom,safe},2}.
  {label,9}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,3,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{1,'+inf'}}}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {get_tuple_element,{x,0},0,{x,2}}.
    {gc_bif,'*',{f,0},3,[{tr,{x,2},{t_integer,{1,'+inf'}}},{integer,4}],{x,2}}.
    {get_tuple_element,{x,0},1,{x,3}}.
    {gc_bif,'+',
            {f,0},
            4,
            [{tr,{x,2},{t_integer,{4,'+inf'}}},{tr,{x,3},{t_integer,any}}],
            {x,2}}.
    {get_tuple_element,{x,0},2,{x,0}}.
    {gc_bif,'div',
            {f,0},
            3,
            [{tr,{x,2},{t_integer,any}},{tr,{x,0},{t_integer,{1,'+inf'}}}],
            {x,0}}.
    {bif,'=:=',
         {f,0},
         [{tr,{x,1},{t_integer,any}},{tr,{x,0},{t_integer,any}}],
         {x,0}}.
    return.


{function, prod, 2, 11}.
  {label,10}.
    {line,[{location,"pidigits.erl",24}]}.
    {func_info,{atom,pidigits},{atom,prod},2}.
  {label,11}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,3,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{1,'+inf'}}}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {get_tuple_element,{x,0},0,{x,2}}.
    {gc_bif,'*',
            {f,0},
            3,
            [{tr,{x,2},{t_integer,{1,'+inf'}}},{integer,10}],
            {x,2}}.
    {get_tuple_element,{x,0},2,{x,3}}.
    {gc_bif,'*',
            {f,0},
            4,
            [{tr,{x,1},{t_integer,any}},{tr,{x,3},{t_integer,{1,'+inf'}}}],
            {x,1}}.
    {get_tuple_element,{x,0},1,{x,0}}.
    {gc_bif,'-',
            {f,0},
            4,
            [{tr,{x,0},{t_integer,any}},{tr,{x,1},{t_integer,any}}],
            {x,0}}.
    {gc_bif,'*',{f,0},4,[{tr,{x,0},{t_integer,any}},{integer,10}],{x,0}}.
    {test_heap,4,4}.
    {put_tuple2,{x,0},{list,[{x,2},{x,0},{x,3}]}}.
    return.


{function, stream, 4, 13}.
  {label,12}.
    {line,[{location,"pidigits.erl",26}]}.
    {func_info,{atom,pidigits},{atom,stream},4}.
  {label,13}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,3,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{1,'+inf'}}}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,{1,'+inf'}}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_integer,any}}]}}.
    {allocate,5,4}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,3},{y,1}}.
    {move,{x,2},{y,2}}.
    {move,{x,1},{y,3}}.
    {move,{x,0},{y,4}}.
    {line,[{location,"pidigits.erl",27}]}.
    {call,1,{f,7}}. % next/1
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {move,{x,0},{y,0}}.
    {move,{x,0},{x,1}}.
    {move,{y,4},{x,0}}.
    {line,[{location,"pidigits.erl",28}]}.
    {call,2,{f,9}}. % safe/2
    {'%',{var_info,{x,0},[{type,{t_atom,[false,true]}}]}}.
    {test,is_eq_exact,{f,14},[{x,0},{atom,true}]}.
    {line,[{location,"pidigits.erl",30}]}.
    {gc_bif,'+',{f,0},0,[{tr,{y,2},{t_integer,{0,'+inf'}}},{integer,1}],{x,0}}.
    {test_heap,4,1}.
    {put_tuple2,{x,1},{list,[{y,0},{x,0},{y,1}]}}.
    {move,{atom,io_worker},{x,0}}.
    send.
    {move,{y,0},{x,1}}.
    {move,{y,4},{x,0}}.
    {move,{y,1},{y,4}}.
    {trim,2,3}.
    {line,[{location,"pidigits.erl",31}]}.
    {call,2,{f,11}}. % prod/2
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,3,true,
                                   #{1 => {t_integer,{10,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{1,'+inf'}}}}}]}}.
    {gc_bif,'+',{f,0},1,[{tr,{y,0},{t_integer,{0,'+inf'}}},{integer,1}],{x,2}}.
    {move,{y,2},{x,3}}.
    {move,{y,1},{x,1}}.
    {call_last,4,{f,13},3}. % stream/4
  {label,14}.
    {line,[{location,"pidigits.erl",32}]}.
    {gc_bif,'*',{f,0},0,[{tr,{y,3},{t_integer,{1,'+inf'}}},{integer,4}],{x,0}}.
    {gc_bif,'+',{f,0},1,[{tr,{x,0},{t_integer,{4,'+inf'}}},{integer,2}],{x,0}}.
    {gc_bif,'*',{f,0},1,[{tr,{y,3},{t_integer,{1,'+inf'}}},{integer,2}],{x,1}}.
    {gc_bif,'+',{f,0},2,[{tr,{x,1},{t_integer,{2,'+inf'}}},{integer,1}],{x,1}}.
    {test_heap,4,2}.
    {put_tuple2,{x,1},{list,[{y,3},{x,0},{x,1}]}}.
    {move,{y,4},{x,0}}.
    {move,{y,1},{y,4}}.
    {trim,2,3}.
    {call,2,{f,5}}. % comp/2
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,3,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{3,'+inf'}}}}}]}}.
    {gc_bif,'+',{f,0},1,[{tr,{y,1},{t_integer,{1,'+inf'}}},{integer,1}],{x,1}}.
    {move,{y,2},{x,3}}.
    {move,{y,0},{x,2}}.
    {call_last,4,{f,13},3}. % stream/4


{function, io_worker, 0, 16}.
  {label,15}.
    {line,[{location,"pidigits.erl",35}]}.
    {func_info,{atom,pidigits},{atom,io_worker},0}.
  {label,16}.
    {allocate,0,0}.
  {label,17}.
    {loop_rec,{f,21},{x,0}}.
    {test,is_tuple,{f,20},[{x,0}]}.
    {test,test_arity,{f,20},[{x,0},3]}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{x,2}}.
    {get_tuple_element,{x,0},2,{x,0}}.
    {test,is_eq_exact,{f,18},[{x,0},{x,2}]}.
    remove_message.
    {line,[{location,"pidigits.erl",38}]}.
    {gc_bif,'rem',{f,0},2,[{x,0},{integer,10}],{x,2}}.
    {gc_bif,'-',{f,0},3,[{integer,10},{tr,{x,2},{t_integer,{-9,9}}}],{x,2}}.
    {gc_bif,'rem',{f,0},3,[{tr,{x,2},{t_integer,{1,19}}},{integer,10}],{x,2}}.
    {test_heap,8,3}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{integer,32},{x,0},{x,0}}.
    {put_list,{x,2},{x,0},{x,0}}.
    {put_list,{x,1},{x,0},{x,1}}.
    {move,{literal,"~w~.*c\t:~w~n"},{x,0}}.
    {line,[{location,"pidigits.erl",39}]}.
    {call_ext,2,{extfunc,io,fwrite,2}}.
    {move,{integer,0},{x,0}}.
    {line,[{location,"pidigits.erl",40}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.
  {label,18}.
    {gc_bif,'rem',{f,19},3,[{x,2},{integer,10}],{x,0}}.
    {test,is_eq_exact,{f,19},[{tr,{x,0},{t_integer,{-9,9}}},{integer,0}]}.
    {test_heap,4,3}.
    {move,{x,2},{x,0}}.
    remove_message.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{x,1},{x,0},{x,1}}.
    {move,{literal,"~w\t:~w~n"},{x,0}}.
    {line,[{location,"pidigits.erl",42}]}.
    {call_ext,2,{extfunc,io,fwrite,2}}.
    {call_last,0,{f,16},0}. % io_worker/0
  {label,19}.
    {test_heap,2,2}.
    {move,{x,1},{x,0}}.
    remove_message.
    {put_list,{x,0},nil,{x,1}}.
    {move,{literal,"~w"},{x,0}}.
    {line,[{location,"pidigits.erl",45}]}.
    {call_ext,2,{extfunc,io,fwrite,2}}.
    {call_last,0,{f,16},0}. % io_worker/0
  {label,20}.
    {loop_rec_end,{f,17}}.
  {label,21}.
    {wait,{f,17}}.


{function, module_info, 0, 23}.
  {label,22}.
    {line,[]}.
    {func_info,{atom,pidigits},{atom,module_info},0}.
  {label,23}.
    {move,{atom,pidigits},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 25}.
  {label,24}.
    {line,[]}.
    {func_info,{atom,pidigits},{atom,module_info},1}.
  {label,25}.
    {move,{x,0},{x,1}}.
    {move,{atom,pidigits},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-main/1-fun-0-', 0, 27}.
  {label,26}.
    {line,[{location,"pidigits.erl",13}]}.
    {func_info,{atom,pidigits},{atom,'-main/1-fun-0-'},0}.
  {label,27}.
    {call_only,0,{f,16}}. % io_worker/0
