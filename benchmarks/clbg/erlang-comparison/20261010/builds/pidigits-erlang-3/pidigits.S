{module, pidigits}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 22}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"pidigits.erl",15}]}.
    {func_info,{atom,pidigits},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,3},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,3},[{x,2}]}.
    {allocate,0,2}.
    {move,{x,1},{x,0}}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {call_last,1,{f,2},0}. % main/1
  {label,3}.
    {allocate_heap,1,{alloc,[{words,0},{floats,0},{funs,1}]},1}.
    {move,{x,0},{y,0}}.
    {make_fun3,{f,21},0,0,{x,0},{list,[]}}.
    {line,[{location,"pidigits.erl",17}]}.
    {call_ext,1,{extfunc,erlang,spawn,1}}.
    {move,{x,0},{x,1}}.
    {move,{atom,io_worker},{x,0}}.
    {line,[{location,"pidigits.erl",18}]}.
    {call_ext,2,{extfunc,erlang,register,2}}.
    {move,{integer,0},{x,2}}.
    {move,{integer,1},{x,1}}.
    {move,{y,0},{x,3}}.
    {move,{literal,{1,0,1}},{x,0}}.
    {call_last,4,{f,5},1}. % stream/4


{function, stream, 4, 5}.
  {label,4}.
    {line,[{location,"pidigits.erl",24}]}.
    {func_info,{atom,pidigits},{atom,stream},4}.
  {label,5}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,3,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{1,'+inf'}}}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,{1,'+inf'}}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,{0,'+inf'}}}]}}.
    {get_tuple_element,{x,0},0,{x,4}}.
    {line,[{location,"pidigits.erl",26}]}.
    {gc_bif,'*',{f,0},5,[{tr,{x,4},{t_integer,{1,'+inf'}}},{integer,3}],{x,5}}.
    {get_tuple_element,{x,0},1,{x,6}}.
    {gc_bif,'+',
            {f,0},
            7,
            [{tr,{x,5},{t_integer,{3,'+inf'}}},{tr,{x,6},{t_integer,any}}],
            {x,5}}.
    {get_tuple_element,{x,0},2,{x,7}}.
    {line,[{location,"pidigits.erl",27}]}.
    {gc_bif,'div',
            {f,0},
            8,
            [{tr,{x,5},{t_integer,any}},{tr,{x,7},{t_integer,{1,'+inf'}}}],
            {x,8}}.
    {line,[{location,"pidigits.erl",28}]}.
    {gc_bif,'+',
            {f,0},
            9,
            [{tr,{x,5},{t_integer,any}},{tr,{x,4},{t_integer,{1,'+inf'}}}],
            {x,5}}.
    {gc_bif,'div',
            {f,0},
            9,
            [{tr,{x,5},{t_integer,any}},{tr,{x,7},{t_integer,{1,'+inf'}}}],
            {x,5}}.
    {test,is_eq_exact,
          {f,6},
          [{tr,{x,8},{t_integer,any}},{tr,{x,5},{t_integer,any}}]}.
    {line,[{location,"pidigits.erl",31}]}.
    {gc_bif,'+',{f,0},9,[{tr,{x,2},{t_integer,{0,'+inf'}}},{integer,1}],{x,2}}.
    {allocate_heap,5,4,9}.
    {move,{x,8},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,3},{y,2}}.
    {move,{x,1},{y,3}}.
    {move,{x,0},{y,4}}.
    {put_tuple2,{x,1},{list,[{x,8},{x,2},{x,3}]}}.
    {move,{atom,io_worker},{x,0}}.
    {line,[{location,"pidigits.erl",32}]}.
    send.
    {get_tuple_element,{y,4},0,{x,0}}.
    {line,[{location,"pidigits.erl",22}]}.
    {gc_bif,'*',
            {f,0},
            1,
            [{tr,{x,0},{t_integer,{1,'+inf'}}},{integer,10}],
            {x,0}}.
    {get_tuple_element,{y,4},2,{x,1}}.
    {gc_bif,'*',
            {f,0},
            2,
            [{tr,{y,0},{t_integer,any}},{tr,{x,1},{t_integer,{1,'+inf'}}}],
            {x,2}}.
    {get_tuple_element,{y,4},1,{x,3}}.
    {gc_bif,'-',
            {f,0},
            4,
            [{tr,{x,3},{t_integer,any}},{tr,{x,2},{t_integer,any}}],
            {x,2}}.
    {gc_bif,'*',{f,0},3,[{tr,{x,2},{t_integer,any}},{integer,10}],{x,2}}.
    {test_heap,4,3}.
    {put_tuple2,{x,0},{list,[{x,0},{x,2},{x,1}]}}.
    {move,{y,1},{x,2}}.
    {move,{y,2},{x,3}}.
    {move,{y,3},{x,1}}.
    {call_last,4,{f,5},5}. % stream/4
  {label,6}.
    {line,[{location,"pidigits.erl",35}]}.
    {gc_bif,'*',{f,0},8,[{tr,{x,1},{t_integer,{1,'+inf'}}},{integer,2}],{x,0}}.
    {line,[{location,"pidigits.erl",36}]}.
    {gc_bif,'+',{f,0},8,[{tr,{x,0},{t_integer,{2,'+inf'}}},{integer,1}],{x,5}}.
    {line,[{location,"pidigits.erl",37}]}.
    {gc_bif,'*',
            {f,0},
            8,
            [{tr,{x,4},{t_integer,{1,'+inf'}}},
             {tr,{x,1},{t_integer,{1,'+inf'}}}],
            {x,8}}.
    {gc_bif,'*',{f,0},9,[{tr,{x,0},{t_integer,{2,'+inf'}}},{integer,2}],{x,0}}.
    {gc_bif,'+',{f,0},9,[{tr,{x,0},{t_integer,{4,'+inf'}}},{integer,2}],{x,0}}.
    {gc_bif,'*',
            {f,0},
            9,
            [{tr,{x,4},{t_integer,{1,'+inf'}}},
             {tr,{x,0},{t_integer,{6,'+inf'}}}],
            {x,0}}.
    {gc_bif,'*',
            {f,0},
            9,
            [{tr,{x,6},{t_integer,any}},{tr,{x,5},{t_integer,{3,'+inf'}}}],
            {x,4}}.
    {gc_bif,'+',
            {f,0},
            9,
            [{tr,{x,0},{t_integer,{6,'+inf'}}},{tr,{x,4},{t_integer,any}}],
            {x,0}}.
    {gc_bif,'*',
            {f,0},
            9,
            [{tr,{x,7},{t_integer,{1,'+inf'}}},
             {tr,{x,5},{t_integer,{3,'+inf'}}}],
            {x,4}}.
    {test_heap,4,9}.
    {put_tuple2,{x,0},{list,[{x,8},{x,0},{x,4}]}}.
    {line,[{location,"pidigits.erl",38}]}.
    {gc_bif,'+',{f,0},4,[{tr,{x,1},{t_integer,{1,'+inf'}}},{integer,1}],{x,1}}.
    {call_only,4,{f,5}}. % stream/4


{function, io_worker, 0, 8}.
  {label,7}.
    {line,[{location,"pidigits.erl",42}]}.
    {func_info,{atom,pidigits},{atom,io_worker},0}.
  {label,8}.
    {allocate,0,0}.
    {move,{literal,[out]},{x,1}}.
    {move,{literal,{fd,0,1}},{x,0}}.
    {line,[{location,"pidigits.erl",43}]}.
    {call_ext,2,{extfunc,erlang,open_port,2}}.
    {call_last,1,{f,10},0}. % do_io_work/1


{function, do_io_work, 1, 10}.
  {label,9}.
    {line,[{location,"pidigits.erl",46}]}.
    {func_info,{atom,pidigits},{atom,do_io_work},1}.
  {label,10}.
    {allocate,1,1}.
    {move,{x,0},{y,0}}.
  {label,11}.
    {loop_rec,{f,15},{x,0}}.
    {test,is_tuple,{f,14},[{x,0}]}.
    {test,test_arity,{f,14},[{x,0},3]}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{x,2}}.
    {get_tuple_element,{x,0},2,{x,0}}.
    {test,is_eq_exact,{f,12},[{x,0},{x,2}]}.
    remove_message.
    {line,[{location,"pidigits.erl",49}]}.
    {gc_bif,'rem',{f,0},2,[{x,0},{integer,10}],{x,2}}.
    {gc_bif,'-',{f,0},3,[{integer,10},{tr,{x,2},{t_integer,{-9,9}}}],{x,2}}.
    {gc_bif,'rem',{f,0},3,[{tr,{x,2},{t_integer,{1,19}}},{integer,10}],{x,2}}.
    {test_heap,8,3}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{integer,32},{x,0},{x,0}}.
    {put_list,{x,2},{x,0},{x,0}}.
    {put_list,{x,1},{x,0},{x,1}}.
    {move,{literal,"~w~.*c\t:~w~n"},{x,0}}.
    {line,[{location,"pidigits.erl",50}]}.
    {call_ext,2,{extfunc,io_lib,format,2}}.
    {move,{x,0},{x,1}}.
    {move,{y,0},{x,0}}.
    {trim,1,0}.
    {call_ext,2,{extfunc,erlang,port_command,2}}.
    {move,{integer,0},{x,0}}.
    {line,[{location,"pidigits.erl",51}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.
  {label,12}.
    {gc_bif,'rem',{f,13},3,[{x,2},{integer,10}],{x,0}}.
    {test,is_eq_exact,{f,13},[{tr,{x,0},{t_integer,{-9,9}}},{integer,0}]}.
    {test_heap,4,3}.
    {move,{x,2},{x,0}}.
    remove_message.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{x,1},{x,0},{x,1}}.
    {move,{literal,"~w\t:~w~n"},{x,0}}.
    {line,[{location,"pidigits.erl",53}]}.
    {call_ext,2,{extfunc,io_lib,format,2}}.
    {move,{x,0},{x,1}}.
    {move,{y,0},{x,0}}.
    {call_ext,2,{extfunc,erlang,port_command,2}}.
    {move,{y,0},{x,0}}.
    {call_last,1,{f,10},1}. % do_io_work/1
  {label,13}.
    {test_heap,2,2}.
    {move,{x,1},{x,0}}.
    remove_message.
    {put_list,{x,0},nil,{x,1}}.
    {move,{literal,"~w"},{x,0}}.
    {line,[{location,"pidigits.erl",56}]}.
    {call_ext,2,{extfunc,io_lib,format,2}}.
    {move,{x,0},{x,1}}.
    {move,{y,0},{x,0}}.
    {call_ext,2,{extfunc,erlang,port_command,2}}.
    {move,{y,0},{x,0}}.
    {call_last,1,{f,10},1}. % do_io_work/1
  {label,14}.
    {loop_rec_end,{f,11}}.
  {label,15}.
    {wait,{f,11}}.


{function, module_info, 0, 17}.
  {label,16}.
    {line,[]}.
    {func_info,{atom,pidigits},{atom,module_info},0}.
  {label,17}.
    {move,{atom,pidigits},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 19}.
  {label,18}.
    {line,[]}.
    {func_info,{atom,pidigits},{atom,module_info},1}.
  {label,19}.
    {move,{x,0},{x,1}}.
    {move,{atom,pidigits},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-main/1-anonymous-0-', 0, 21}.
  {label,20}.
    {line,[{location,"pidigits.erl",17}]}.
    {func_info,{atom,pidigits},{atom,'-main/1-anonymous-0-'},0}.
  {label,21}.
    {call_only,0,{f,8}}. % io_worker/0
