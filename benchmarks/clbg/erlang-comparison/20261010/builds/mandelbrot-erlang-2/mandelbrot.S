{module, mandelbrot}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 34}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"mandelbrot.erl",12}]}.
    {func_info,{atom,mandelbrot},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,1},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,1},[{x,2}]}.
    {allocate,1,2}.
    {move,{x,1},{y,0}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"mandelbrot.erl",13}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {test_heap,8,1}.
    {put_list,{y,0},{literal,["\n"]},{x,1}}.
    {put_list,{literal," "},{x,1},{x,1}}.
    {put_list,{y,0},{x,1},{x,1}}.
    {put_list,{literal,"P4\n"},{x,1},{x,1}}.
    {move,{x,0},{y,0}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"mandelbrot.erl",14}]}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {test_heap,{alloc,[{words,1},{floats,0},{funs,1}]},0}.
    {make_fun3,{f,33},0,0,{x,0},{list,[{y,0}]}}.
    {move,{y,0},{x,1}}.
    {call_last,2,{f,4},1}. % spawn_proc_chain/2


{function, spawn_proc_chain, 2, 4}.
  {label,3}.
    {line,[{location,"mandelbrot.erl",21}]}.
    {func_info,{atom,mandelbrot},{atom,spawn_proc_chain},2}.
  {label,4}.
    {'%',{var_info,{x,0},[{type,{t_fun,2,{'-main/1-fun-0-',3},any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {test_heap,{alloc,[{words,0},{floats,0},{funs,1}]},2}.
    {make_fun3,{f,29},0,0,{x,2},{list,[]}}.
    {'%',{var_info,{x,2},[{fun_type,pid}]}}.
    {move,{x,1},{x,3}}.
    {move,{x,0},{x,1}}.
    {move,{x,2},{x,0}}.
    {move,{atom,first},{x,2}}.
    {call_only,4,{f,29}}. % '-spawn_proc_chain/2-fun-1-'/4


{function, do_spawn_proc_chain, 4, 6}.
  {label,5}.
    {line,[{location,"mandelbrot.erl",25}]}.
    {func_info,{atom,mandelbrot},{atom,do_spawn_proc_chain},4}.
  {label,6}.
    {test,is_eq_exact,{f,9},[{x,3},{x,2}]}.
    {allocate,0,0}.
  {label,7}.
    {loop_rec,{f,8},{x,0}}.
    remove_message.
    {move,{integer,0},{x,0}}.
    {line,[{location,"mandelbrot.erl",26}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.
  {label,8}.
    {wait,{f,7}}.
  {label,9}.
    {test,is_eq_exact,{f,10},[{x,2},{atom,first}]}.
    {allocate,2,4}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,1},{y,1}}.
    {move,{x,0},{x,4}}.
    {move,{integer,1},{x,2}}.
    {line,[{location,"mandelbrot.erl",28}]}.
    {call_fun,4}.
    {bif,self,{f,0},[],{x,1}}.
    {move,{x,0},{y,0}}.
    {move,{x,1},{x,0}}.
    {move,{atom,done},{x,1}}.
    {line,[{location,"mandelbrot.erl",30}]}.
    send.
    {move,{integer,0},{x,1}}.
    {move,{y,1},{x,2}}.
    {move,{y,0},{x,0}}.
    {line,[{location,"mandelbrot.erl",32}]}.
    {call_fun,2}.
    {deallocate,2}.
    return.
  {label,10}.
    {line,[{location,"mandelbrot.erl",34}]}.
    {gc_bif,'+',{f,0},4,[{x,2},{integer,1}],{x,4}}.
    {allocate,2,5}.
    {move,{x,2},{y,0}}.
    {move,{x,1},{y,1}}.
    {move,{x,4},{x,2}}.
    {move,{x,0},{x,4}}.
    {call_fun,4}.
    {move,{y,0},{x,1}}.
    {move,{y,1},{x,2}}.
    {line,[{location,"mandelbrot.erl",35}]}.
    {call_fun,2}.
    {deallocate,2}.
    return.


{function, row, 8, 12}.
  {label,11}.
    {line,[{location,"mandelbrot.erl",39}]}.
    {func_info,{atom,mandelbrot},{atom,row},8}.
  {label,12}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,4},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,5},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,6},[{type,{t_list,{t_integer,{0,'+inf'}},nil}}]}}.
    {'%',{var_info,{x,7},[{type,{t_integer,any}}]}}.
    {test,is_eq_exact,
          {f,16},
          [{tr,{x,2},{t_integer,{0,'+inf'}}},{tr,{x,1},{t_integer,any}}]}.
    {allocate,2,8}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,0},{y,1}}.
    {test,is_eq_exact,{f,13},[{tr,{x,7},{t_integer,any}},{integer,7}]}.
    {move,{x,6},{x,0}}.
    {line,[{location,"mandelbrot.erl",41}]}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {move,{x,0},{y,0}}.
    {jump,{f,14}}.
  {label,13}.
    {line,[{location,"mandelbrot.erl",42}]}.
    {gc_bif,'+',{f,0},8,[{tr,{x,7},{t_integer,any}},{integer,1}],{x,0}}.
    {gc_bif,'bsl',
            {f,0},
            7,
            [{tr,{x,5},{t_integer,{0,'+inf'}}},{tr,{x,0},{t_integer,any}}],
            {x,0}}.
    {test_heap,2,7}.
    {put_list,{x,0},{x,6},{x,0}}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {move,{x,0},{y,0}}.
  {label,14}.
    {loop_rec,{f,15},{x,0}}.
    remove_message.
    {move,{y,0},{x,0}}.
    {trim,1,1}.
    {line,[{location,"mandelbrot.erl",46}]}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {move,{atom,done},{x,1}}.
    {move,{y,0},{x,0}}.
    {line,[{location,"mandelbrot.erl",47}]}.
    send.
    {deallocate,1}.
    return.
  {label,15}.
    {wait,{f,14}}.
  {label,16}.
    {test,is_eq_exact,{f,17},[{tr,{x,7},{t_integer,any}},{integer,0}]}.
    {line,[{location,"mandelbrot.erl",50}]}.
    {gc_bif,'+',{f,0},7,[{tr,{x,2},{t_integer,{0,'+inf'}}},{integer,1}],{x,7}}.
    {gc_bif,'bsl',
            {f,0},
            8,
            [{tr,{x,5},{t_integer,{0,'+inf'}}},{integer,1}],
            {x,5}}.
    {gc_bif,'+',
            {f,0},
            8,
            [{tr,{x,2},{t_integer,{0,'+inf'}}},
             {tr,{x,2},{t_integer,{0,'+inf'}}}],
            {x,2}}.
    {fconv,{tr,{x,2},{t_integer,{0,'+inf'}}},{fr,0}}.
    {fconv,{tr,{x,4},{t_integer,any}},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,-1.5},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {allocate_heap,7,{alloc,[{words,0},{floats,1},{funs,0}]},8}.
    {move,{x,7},{y,0}}.
    {move,{x,5},{y,1}}.
    {move,{x,6},{y,2}}.
    {move,{x,4},{y,3}}.
    {move,{x,3},{y,4}}.
    {move,{x,1},{y,5}}.
    {move,{x,0},{y,6}}.
    {fmove,{fr,0},{x,1}}.
    {move,{x,3},{x,2}}.
    {move,{integer,50},{x,0}}.
    {call,3,{f,19}}. % m/3
    {'%',{var_info,{x,0},[{type,{t_integer,{0,1}}}]}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,1},{t_integer,{0,'+inf'}}},{tr,{x,0},{t_integer,{0,1}}}],
            {x,0}}.
    {test_heap,2,1}.
    {put_list,{x,0},{y,2},{x,6}}.
    {move,{y,0},{x,2}}.
    {move,{y,3},{x,4}}.
    {move,{y,4},{x,3}}.
    {move,{integer,0},{x,5}}.
    {move,{y,5},{x,1}}.
    {move,{integer,7},{x,7}}.
    {move,{y,6},{x,0}}.
    {call_last,8,{f,12},7}. % row/8
  {label,17}.
    {line,[{location,"mandelbrot.erl",53}]}.
    {gc_bif,'+',{f,0},8,[{tr,{x,2},{t_integer,{0,'+inf'}}},{integer,1}],{x,8}}.
    {gc_bif,'bsl',
            {f,0},
            9,
            [{tr,{x,5},{t_integer,{0,'+inf'}}},{integer,1}],
            {x,5}}.
    {gc_bif,'+',
            {f,0},
            9,
            [{tr,{x,2},{t_integer,{0,'+inf'}}},
             {tr,{x,2},{t_integer,{0,'+inf'}}}],
            {x,2}}.
    {fconv,{tr,{x,2},{t_integer,{0,'+inf'}}},{fr,0}}.
    {fconv,{tr,{x,4},{t_integer,any}},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,-1.5},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {allocate_heap,8,{alloc,[{words,0},{floats,1},{funs,0}]},9}.
    {move,{x,8},{y,0}}.
    {move,{x,5},{y,1}}.
    {move,{x,7},{y,2}}.
    {move,{x,6},{y,3}}.
    {move,{x,4},{y,4}}.
    {move,{x,3},{y,5}}.
    {move,{x,1},{y,6}}.
    {move,{x,0},{y,7}}.
    {fmove,{fr,0},{x,1}}.
    {move,{x,3},{x,2}}.
    {move,{integer,50},{x,0}}.
    {call,3,{f,19}}. % m/3
    {'%',{var_info,{x,0},[{type,{t_integer,{0,1}}}]}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,1},{t_integer,{0,'+inf'}}},{tr,{x,0},{t_integer,{0,1}}}],
            {x,0}}.
    {gc_bif,'-',{f,0},1,[{tr,{y,2},{t_integer,any}},{integer,1}],{x,7}}.
    {move,{y,0},{x,2}}.
    {move,{y,4},{x,4}}.
    {move,{y,5},{x,3}}.
    {move,{x,0},{x,5}}.
    {move,{y,3},{x,6}}.
    {move,{y,6},{x,1}}.
    {move,{y,7},{x,0}}.
    {call_last,8,{f,12},8}. % row/8


{function, m, 3, 19}.
  {label,18}.
    {line,[{location,"mandelbrot.erl",57}]}.
    {func_info,{atom,mandelbrot},{atom,m},3}.
  {label,19}.
    {'%',{var_info,{x,0},[{type,{t_integer,{50,50}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_float,any}}]}}.
    {move,{x,1},{x,3}}.
    {move,{x,2},{x,4}}.
    {move,{integer,49},{x,0}}.
    {call_only,5,{f,21}}. % m/5


{function, m, 5, 21}.
  {label,20}.
    {line,[{location,"mandelbrot.erl",59}]}.
    {func_info,{atom,mandelbrot},{atom,m},5}.
  {label,21}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,3},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,4},[{type,{t_float,any}}]}}.
    {fmove,{x,1},{fr,0}}.
    {line,[{location,"mandelbrot.erl",60}]}.
    {bif,fmul,{f,0},[{fr,0},{fr,0}],{fr,0}}.
    {fmove,{x,2},{fr,1}}.
    {bif,fmul,{f,0},[{fr,1},{fr,1}],{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,2}}.
    {test_heap,{alloc,[{words,0},{floats,3},{funs,0}]},5}.
    {fmove,{fr,2},{x,5}}.
    {fmove,{fr,0},{x,6}}.
    {fmove,{fr,1},{x,7}}.
    {test,is_lt,{f,22},[{float,4.0},{tr,{x,5},{t_float,any}}]}.
    {move,{integer,0},{x,0}}.
    return.
  {label,22}.
    {test,is_ge,{f,23},[{tr,{x,0},{t_integer,any}},{integer,1}]}.
    {line,[{location,"mandelbrot.erl",61}]}.
    {gc_bif,'-',{f,0},8,[{tr,{x,0},{t_integer,{1,'+inf'}}},{integer,1}],{x,0}}.
    {fmove,{x,6},{fr,0}}.
    {fmove,{x,7},{fr,1}}.
    {bif,fsub,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{x,3},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{x,1},{fr,1}}.
    {fmove,{float,2.0},{fr,2}}.
    {bif,fmul,{f,0},[{fr,1},{fr,2}],{fr,1}}.
    {fmove,{x,2},{fr,2}}.
    {bif,fmul,{f,0},[{fr,1},{fr,2}],{fr,1}}.
    {fmove,{x,4},{fr,2}}.
    {bif,fadd,{f,0},[{fr,1},{fr,2}],{fr,1}}.
    {test_heap,{alloc,[{words,0},{floats,2},{funs,0}]},5}.
    {fmove,{fr,0},{x,1}}.
    {fmove,{fr,1},{x,2}}.
    {call_only,5,{f,21}}. % m/5
  {label,23}.
    {move,{integer,1},{x,0}}.
    return.


{function, module_info, 0, 25}.
  {label,24}.
    {line,[]}.
    {func_info,{atom,mandelbrot},{atom,module_info},0}.
  {label,25}.
    {move,{atom,mandelbrot},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 27}.
  {label,26}.
    {line,[]}.
    {func_info,{atom,mandelbrot},{atom,module_info},1}.
  {label,27}.
    {move,{x,0},{x,1}}.
    {move,{atom,mandelbrot},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-spawn_proc_chain/2-fun-1-', 4, 29}.
  {label,28}.
    {line,[{location,"mandelbrot.erl",22}]}.
    {func_info,{atom,mandelbrot},{atom,'-spawn_proc_chain/2-fun-1-'},4}.
  {label,29}.
    {test_heap,{alloc,[{words,4},{floats,0},{funs,1}]},4}.
    {make_fun3,{f,31},0,0,{x,0},{list,[{x,3},{x,2},{x,1},{x,0}]}}.
    {call_ext_only,1,{extfunc,erlang,spawn,1}}.


{function, '-spawn_proc_chain/2-fun-0-', 4, 31}.
  {label,30}.
    {line,[{location,"mandelbrot.erl",22}]}.
    {func_info,{atom,mandelbrot},{atom,'-spawn_proc_chain/2-fun-0-'},4}.
  {label,31}.
    {swap,{x,1},{x,2}}.
    {swap,{x,0},{x,3}}.
    {call_only,4,{f,6}}. % do_spawn_proc_chain/4


{function, '-main/1-fun-0-', 3, 33}.
  {label,32}.
    {line,[{location,"mandelbrot.erl",17}]}.
    {func_info,{atom,mandelbrot},{atom,'-main/1-fun-0-'},3}.
  {label,33}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {gc_bif,'-',{f,0},3,[{tr,{x,2},{t_integer,any}},{integer,1}],{x,3}}.
    {gc_bif,'*',{f,0},4,[{x,1},{integer,2}],{x,1}}.
    {fconv,{tr,{x,1},{t_number,any}},{fr,0}}.
    {fconv,{tr,{x,2},{t_integer,any}},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,-1.0},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},4}.
    {fmove,{fr,0},{x,1}}.
    {move,{integer,0},{x,5}}.
    {move,nil,{x,6}}.
    {move,{x,2},{x,4}}.
    {move,{integer,7},{x,7}}.
    {move,{integer,0},{x,2}}.
    {swap,{x,1},{x,3}}.
    {call_only,8,{f,12}}. % row/8
