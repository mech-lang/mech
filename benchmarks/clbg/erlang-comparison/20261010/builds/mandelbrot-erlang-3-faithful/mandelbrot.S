{module, mandelbrot}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 53}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"mandelbrot.erl",13}]}.
    {func_info,{atom,mandelbrot},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,1},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,1},[{x,2}]}.
    {allocate,2,2}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,1},{y,1}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"mandelbrot.erl",14}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {move,{x,0},{y,0}}.
    {line,[{location,"mandelbrot.erl",18}]}.
    {call,0,{f,28}}. % print_start/0
    {'%',{var_info,{x,0},[{type,pid}]}}.
    {test_heap,8,1}.
    {put_list,{y,1},{literal,["\n"]},{x,1}}.
    {put_list,{literal," "},{x,1},{x,1}}.
    {put_list,{y,1},{x,1},{x,1}}.
    {put_list,{literal,"P4\n"},{x,1},{x,1}}.
    {move,{x,0},{y,1}}.
    {line,[{location,"mandelbrot.erl",19}]}.
    {call,2,{f,26}}. % print/2
    {test_heap,{alloc,[{words,2},{floats,0},{funs,1}]},0}.
    {make_fun3,{f,52},0,0,{x,1},{list,[{y,0},{y,1}]}}.
    {move,{y,0},{x,2}}.
    {move,{y,1},{x,0}}.
    {call_last,3,{f,4},2}. % spawn_proc_chain/3


{function, spawn_proc_chain, 3, 4}.
  {label,3}.
    {line,[{location,"mandelbrot.erl",26}]}.
    {func_info,{atom,mandelbrot},{atom,spawn_proc_chain},3}.
  {label,4}.
    {'%',{var_info,{x,0},[{type,pid}]}}.
    {'%',{var_info,{x,1},[{type,{t_fun,2,{'-main/1-fun-0-',4},any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {test_heap,{alloc,[{words,0},{floats,0},{funs,1}]},3}.
    {make_fun3,{f,48},0,0,{x,3},{list,[]}}.
    {'%',{var_info,{x,3},[{fun_type,pid}]}}.
    {move,{x,2},{x,4}}.
    {move,{x,1},{x,2}}.
    {move,{x,3},{x,1}}.
    {move,{atom,first},{x,3}}.
    {call_only,5,{f,48}}. % '-spawn_proc_chain/3-fun-1-'/5


{function, do_spawn_proc_chain, 5, 6}.
  {label,5}.
    {line,[{location,"mandelbrot.erl",30}]}.
    {func_info,{atom,mandelbrot},{atom,do_spawn_proc_chain},5}.
  {label,6}.
    {test,is_eq_exact,{f,10},[{x,4},{x,3}]}.
    {allocate,1,1}.
    {move,{x,0},{y,0}}.
  {label,7}.
    {loop_rec,{f,9},{x,0}}.
    {test,is_eq_exact,{f,8},[{x,0},{atom,done}]}.
    remove_message.
    {move,{y,0},{x,0}}.
    {trim,1,0}.
    {line,[{location,"mandelbrot.erl",33}]}.
    {call,1,{f,30}}. % print_stop/1
    {'%',{var_info,{x,0},[{type,{t_atom,[ok]}}]}}.
    {move,{integer,0},{x,0}}.
    {line,[{location,"mandelbrot.erl",34}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.
  {label,8}.
    {loop_rec_end,{f,7}}.
  {label,9}.
    {wait,{f,7}}.
  {label,10}.
    {test,is_eq_exact,{f,11},[{x,3},{atom,first}]}.
    {allocate,2,5}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,2},{y,1}}.
    {move,{x,1},{x,5}}.
    {move,{integer,1},{x,3}}.
    {line,[{location,"mandelbrot.erl",36}]}.
    {call_fun,5}.
    {bif,self,{f,0},[],{x,1}}.
    {move,{x,0},{y,0}}.
    {move,{x,1},{x,0}}.
    {move,{atom,done},{x,1}}.
    {line,[{location,"mandelbrot.erl",38}]}.
    send.
    {move,{integer,0},{x,1}}.
    {move,{y,1},{x,2}}.
    {move,{y,0},{x,0}}.
    {line,[{location,"mandelbrot.erl",40}]}.
    {call_fun,2}.
    {deallocate,2}.
    return.
  {label,11}.
    {line,[{location,"mandelbrot.erl",42}]}.
    {gc_bif,'+',{f,0},5,[{x,3},{integer,1}],{x,5}}.
    {allocate,2,6}.
    {move,{x,3},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,5},{x,3}}.
    {move,{x,1},{x,5}}.
    {call_fun,5}.
    {move,{y,0},{x,1}}.
    {move,{y,1},{x,2}}.
    {line,[{location,"mandelbrot.erl",43}]}.
    {call_fun,2}.
    {deallocate,2}.
    return.


{function, row, 9, 13}.
  {label,12}.
    {line,[{location,"mandelbrot.erl",47}]}.
    {func_info,{atom,mandelbrot},{atom,row},9}.
  {label,13}.
    {'%',{var_info,{x,0},[{type,pid}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,3},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,4},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,5},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,6},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,7},[{type,{t_list,{t_integer,{0,'+inf'}},nil}}]}}.
    {'%',{var_info,{x,8},[{type,{t_integer,any}}]}}.
    {test,is_eq_exact,
          {f,17},
          [{tr,{x,3},{t_integer,{0,'+inf'}}},{tr,{x,2},{t_integer,any}}]}.
    {allocate,3,9}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,1},{y,1}}.
    {move,{x,0},{y,2}}.
    {test,is_eq_exact,{f,14},[{tr,{x,8},{t_integer,any}},{integer,7}]}.
    {move,{x,7},{x,0}}.
    {line,[{location,"mandelbrot.erl",49}]}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {move,{x,0},{y,0}}.
    {jump,{f,15}}.
  {label,14}.
    {line,[{location,"mandelbrot.erl",50}]}.
    {gc_bif,'+',{f,0},9,[{tr,{x,8},{t_integer,any}},{integer,1}],{x,0}}.
    {gc_bif,'bsl',
            {f,0},
            8,
            [{tr,{x,6},{t_integer,{0,'+inf'}}},{tr,{x,0},{t_integer,any}}],
            {x,0}}.
    {test_heap,2,8}.
    {put_list,{x,0},{x,7},{x,0}}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {move,{x,0},{y,0}}.
  {label,15}.
    {loop_rec,{f,16},{x,0}}.
    remove_message.
    {move,{y,0},{x,1}}.
    {move,{y,2},{x,0}}.
    {move,{y,1},{y,2}}.
    {trim,2,1}.
    {line,[{location,"mandelbrot.erl",54}]}.
    {call,2,{f,26}}. % print/2
    {move,{atom,done},{x,1}}.
    {move,{y,0},{x,0}}.
    {line,[{location,"mandelbrot.erl",55}]}.
    send.
    {deallocate,1}.
    return.
  {label,16}.
    {wait,{f,15}}.
  {label,17}.
    {test,is_eq_exact,{f,18},[{tr,{x,8},{t_integer,any}},{integer,0}]}.
    {line,[{location,"mandelbrot.erl",58}]}.
    {gc_bif,'+',{f,0},8,[{tr,{x,3},{t_integer,{0,'+inf'}}},{integer,1}],{x,8}}.
    {gc_bif,'bsl',
            {f,0},
            9,
            [{tr,{x,6},{t_integer,{0,'+inf'}}},{integer,1}],
            {x,6}}.
    {gc_bif,'+',
            {f,0},
            9,
            [{tr,{x,3},{t_integer,{0,'+inf'}}},
             {tr,{x,3},{t_integer,{0,'+inf'}}}],
            {x,3}}.
    {fconv,{tr,{x,3},{t_integer,{0,'+inf'}}},{fr,0}}.
    {fconv,{tr,{x,5},{t_integer,any}},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,-1.5},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {allocate_heap,8,{alloc,[{words,0},{floats,1},{funs,0}]},9}.
    {move,{x,8},{y,0}}.
    {move,{x,6},{y,1}}.
    {move,{x,7},{y,2}}.
    {move,{x,5},{y,3}}.
    {move,{x,4},{y,4}}.
    {move,{x,2},{y,5}}.
    {move,{x,1},{y,6}}.
    {move,{x,0},{y,7}}.
    {fmove,{fr,0},{x,1}}.
    {move,{x,4},{x,2}}.
    {move,{integer,50},{x,0}}.
    {call,3,{f,20}}. % m/3
    {'%',{var_info,{x,0},[{type,{t_integer,{0,1}}}]}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,1},{t_integer,{0,'+inf'}}},{tr,{x,0},{t_integer,{0,1}}}],
            {x,0}}.
    {test_heap,2,1}.
    {put_list,{x,0},{y,2},{x,7}}.
    {move,{y,0},{x,3}}.
    {move,{y,3},{x,5}}.
    {move,{y,4},{x,4}}.
    {move,{y,5},{x,2}}.
    {move,{integer,0},{x,6}}.
    {move,{y,6},{x,1}}.
    {move,{integer,7},{x,8}}.
    {move,{y,7},{x,0}}.
    {call_last,9,{f,13},8}. % row/9
  {label,18}.
    {line,[{location,"mandelbrot.erl",61}]}.
    {gc_bif,'+',{f,0},9,[{tr,{x,3},{t_integer,{0,'+inf'}}},{integer,1}],{x,9}}.
    {gc_bif,'bsl',
            {f,0},
            10,
            [{tr,{x,6},{t_integer,{0,'+inf'}}},{integer,1}],
            {x,6}}.
    {gc_bif,'+',
            {f,0},
            10,
            [{tr,{x,3},{t_integer,{0,'+inf'}}},
             {tr,{x,3},{t_integer,{0,'+inf'}}}],
            {x,3}}.
    {fconv,{tr,{x,3},{t_integer,{0,'+inf'}}},{fr,0}}.
    {fconv,{tr,{x,5},{t_integer,any}},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,-1.5},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {allocate_heap,9,{alloc,[{words,0},{floats,1},{funs,0}]},10}.
    {move,{x,9},{y,0}}.
    {move,{x,6},{y,1}}.
    {move,{x,8},{y,2}}.
    {move,{x,7},{y,3}}.
    {move,{x,5},{y,4}}.
    {move,{x,4},{y,5}}.
    {move,{x,2},{y,6}}.
    {move,{x,1},{y,7}}.
    {move,{x,0},{y,8}}.
    {fmove,{fr,0},{x,1}}.
    {move,{x,4},{x,2}}.
    {move,{integer,50},{x,0}}.
    {call,3,{f,20}}. % m/3
    {'%',{var_info,{x,0},[{type,{t_integer,{0,1}}}]}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,1},{t_integer,{0,'+inf'}}},{tr,{x,0},{t_integer,{0,1}}}],
            {x,0}}.
    {gc_bif,'-',{f,0},1,[{tr,{y,2},{t_integer,any}},{integer,1}],{x,8}}.
    {move,{y,0},{x,3}}.
    {move,{y,4},{x,5}}.
    {move,{y,5},{x,4}}.
    {move,{y,6},{x,2}}.
    {move,{x,0},{x,6}}.
    {move,{y,3},{x,7}}.
    {move,{y,7},{x,1}}.
    {move,{y,8},{x,0}}.
    {call_last,9,{f,13},9}. % row/9


{function, m, 3, 20}.
  {label,19}.
    {line,[{location,"mandelbrot.erl",65}]}.
    {func_info,{atom,mandelbrot},{atom,m},3}.
  {label,20}.
    {'%',{var_info,{x,0},[{type,{t_integer,{50,50}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_float,any}}]}}.
    {move,{x,1},{x,3}}.
    {move,{x,2},{x,4}}.
    {move,{integer,49},{x,0}}.
    {call_only,5,{f,22}}. % m/5


{function, m, 5, 22}.
  {label,21}.
    {line,[{location,"mandelbrot.erl",67}]}.
    {func_info,{atom,mandelbrot},{atom,m},5}.
  {label,22}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,3},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,4},[{type,{t_float,any}}]}}.
    {fmove,{x,1},{fr,0}}.
    {line,[{location,"mandelbrot.erl",68}]}.
    {bif,fmul,{f,0},[{fr,0},{fr,0}],{fr,0}}.
    {fmove,{x,2},{fr,1}}.
    {bif,fmul,{f,0},[{fr,1},{fr,1}],{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,2}}.
    {test_heap,{alloc,[{words,0},{floats,3},{funs,0}]},5}.
    {fmove,{fr,2},{x,5}}.
    {fmove,{fr,0},{x,6}}.
    {fmove,{fr,1},{x,7}}.
    {test,is_lt,{f,23},[{float,4.0},{tr,{x,5},{t_float,any}}]}.
    {move,{integer,0},{x,0}}.
    return.
  {label,23}.
    {test,is_ge,{f,24},[{tr,{x,0},{t_integer,any}},{integer,1}]}.
    {line,[{location,"mandelbrot.erl",69}]}.
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
    {call_only,5,{f,22}}. % m/5
  {label,24}.
    {move,{integer,1},{x,0}}.
    return.


{function, print, 2, 26}.
  {label,25}.
    {line,[{location,"mandelbrot.erl",76}]}.
    {func_info,{atom,mandelbrot},{atom,print},2}.
  {label,26}.
    {'%',{var_info,{x,0},[{type,pid}]}}.
    {'%',{var_info,{x,1},[{type,{t_list,any,nil}}]}}.
    {allocate,0,2}.
    {line,[{location,"mandelbrot.erl",77}]}.
    send.
    {deallocate,0}.
    return.


{function, print_start, 0, 28}.
  {label,27}.
    {line,[{location,"mandelbrot.erl",79}]}.
    {func_info,{atom,mandelbrot},{atom,print_start},0}.
  {label,28}.
    {test_heap,{alloc,[{words,0},{floats,0},{funs,1}]},0}.
    {make_fun3,{f,46},0,0,{x,0},{list,[]}}.
    {line,[{location,"mandelbrot.erl",80}]}.
    {call_ext_only,1,{extfunc,erlang,spawn,1}}.


{function, print_stop, 1, 30}.
  {label,29}.
    {line,[{location,"mandelbrot.erl",82}]}.
    {func_info,{atom,mandelbrot},{atom,print_stop},1}.
  {label,30}.
    {allocate_heap,0,3,1}.
    {bif,self,{f,0},[],{x,1}}.
    {put_tuple2,{x,1},{list,[{x,1},{atom,stop}]}}.
    {line,[{location,"mandelbrot.erl",83}]}.
    send.
  {label,31}.
    {loop_rec,{f,33},{x,0}}.
    {test,is_eq_exact,{f,32},[{x,0},{atom,stopped}]}.
    remove_message.
    {move,{atom,ok},{x,0}}.
    {deallocate,0}.
    return.
  {label,32}.
    {loop_rec_end,{f,31}}.
  {label,33}.
    {wait,{f,31}}.


{function, do_print_start, 0, 35}.
  {label,34}.
    {line,[{location,"mandelbrot.erl",86}]}.
    {func_info,{atom,mandelbrot},{atom,do_print_start},0}.
  {label,35}.
    {allocate,0,0}.
    {move,{literal,[out]},{x,1}}.
    {move,{literal,{fd,0,1}},{x,0}}.
    {line,[{location,"mandelbrot.erl",87}]}.
    {call_ext,2,{extfunc,erlang,open_port,2}}.
    {call_last,1,{f,37},0}. % print_loop/1


{function, print_loop, 1, 37}.
  {label,36}.
    {line,[{location,"mandelbrot.erl",90}]}.
    {func_info,{atom,mandelbrot},{atom,print_loop},1}.
  {label,37}.
    {allocate,1,1}.
    {move,{x,0},{y,0}}.
  {label,38}.
    {loop_rec,{f,40},{x,0}}.
    {test,is_tuple,{f,39},[{x,0}]}.
    {test,test_arity,{f,39},[{x,0},2]}.
    {get_tuple_element,{x,0},1,{x,1}}.
    {test,is_eq_exact,{f,39},[{x,1},{atom,stop}]}.
    remove_message.
    {get_tuple_element,{x,0},0,{x,0}}.
    {move,{atom,stopped},{x,1}}.
    {line,[{location,"mandelbrot.erl",92}]}.
    send.
    {deallocate,1}.
    return.
  {label,39}.
    {move,{x,0},{x,1}}.
    remove_message.
    {move,{y,0},{x,0}}.
    {line,[{location,"mandelbrot.erl",94}]}.
    {call_ext,2,{extfunc,erlang,port_command,2}}.
    {move,{y,0},{x,0}}.
    {call_last,1,{f,37},1}. % print_loop/1
  {label,40}.
    {wait,{f,38}}.


{function, module_info, 0, 42}.
  {label,41}.
    {line,[]}.
    {func_info,{atom,mandelbrot},{atom,module_info},0}.
  {label,42}.
    {move,{atom,mandelbrot},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 44}.
  {label,43}.
    {line,[]}.
    {func_info,{atom,mandelbrot},{atom,module_info},1}.
  {label,44}.
    {move,{x,0},{x,1}}.
    {move,{atom,mandelbrot},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-print_start/0-fun-0-', 0, 46}.
  {label,45}.
    {line,[{location,"mandelbrot.erl",80}]}.
    {func_info,{atom,mandelbrot},{atom,'-print_start/0-fun-0-'},0}.
  {label,46}.
    {call_only,0,{f,35}}. % do_print_start/0


{function, '-spawn_proc_chain/3-fun-1-', 5, 48}.
  {label,47}.
    {line,[{location,"mandelbrot.erl",27}]}.
    {func_info,{atom,mandelbrot},{atom,'-spawn_proc_chain/3-fun-1-'},5}.
  {label,48}.
    {test_heap,{alloc,[{words,5},{floats,0},{funs,1}]},5}.
    {make_fun3,{f,50},0,0,{x,0},{list,[{x,4},{x,3},{x,2},{x,1},{x,0}]}}.
    {call_ext_only,1,{extfunc,erlang,spawn,1}}.


{function, '-spawn_proc_chain/3-fun-0-', 5, 50}.
  {label,49}.
    {line,[{location,"mandelbrot.erl",27}]}.
    {func_info,{atom,mandelbrot},{atom,'-spawn_proc_chain/3-fun-0-'},5}.
  {label,50}.
    {swap,{x,1},{x,3}}.
    {swap,{x,0},{x,4}}.
    {call_only,5,{f,6}}. % do_spawn_proc_chain/5


{function, '-main/1-fun-0-', 4, 52}.
  {label,51}.
    {line,[{location,"mandelbrot.erl",22}]}.
    {func_info,{atom,mandelbrot},{atom,'-main/1-fun-0-'},4}.
  {label,52}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,3},[{type,pid}]}}.
    {gc_bif,'*',{f,0},4,[{x,1},{integer,2}],{x,1}}.
    {fconv,{tr,{x,1},{t_number,any}},{fr,0}}.
    {fconv,{tr,{x,2},{t_integer,any}},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,-1.0},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},4}.
    {fmove,{fr,0},{x,4}}.
    {move,{integer,0},{x,6}}.
    {move,{x,2},{x,5}}.
    {move,nil,{x,7}}.
    {move,{x,0},{x,1}}.
    {move,{integer,7},{x,8}}.
    {move,{x,3},{x,0}}.
    {move,{integer,0},{x,3}}.
    {call_only,9,{f,13}}. % row/9
