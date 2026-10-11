{module, mandelbrot}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 28}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"mandelbrot.erl",12}]}.
    {func_info,{atom,mandelbrot},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,1},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,1},[{x,2}]}.
    {allocate,2,2}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,1},{y,1}}.
    {move,{literal,[{encoding,latin1}]},{x,1}}.
    {move,{atom,standard_io},{x,0}}.
    {line,[{location,"mandelbrot.erl",13}]}.
    {call_ext,2,{extfunc,io,setopts,2}}.
    {move,{y,1},{x,0}}.
    {line,[{location,"mandelbrot.erl",14}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {test_heap,8,1}.
    {put_list,{y,1},{literal,["\n"]},{x,1}}.
    {put_list,{literal," "},{x,1},{x,1}}.
    {put_list,{y,1},{x,1},{x,1}}.
    {put_list,{literal,"P4\n"},{x,1},{x,1}}.
    {move,{x,0},{y,1}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"mandelbrot.erl",15}]}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {test_heap,{alloc,[{words,1},{floats,0},{funs,1}]},0}.
    {make_fun3,{f,25},0,0,{y,0},{list,[{y,1}]}}.
    {'%',{var_info,{y,0},[{fun_type,pid}]}}.
    {line,[{location,"mandelbrot.erl",19}]}.
    {gc_bif,'-',{f,0},0,[{tr,{y,1},{t_integer,any}},{integer,1}],{x,1}}.
    {init_yregs,{list,[{y,1}]}}.
    {move,{integer,0},{x,0}}.
    {call_ext,2,{extfunc,lists,seq,2}}.
    {move,{x,0},{x,1}}.
    {move,{y,0},{x,0}}.
    {init_yregs,{list,[{y,0}]}}.
    {call_ext,2,{extfunc,lists,map,2}}.
    {line,[{location,"mandelbrot.erl",22}]}.
    {bif,hd,{f,0},[{tr,{x,0},{t_list,pid,nil}}],{y,1}}.
    {test_heap,2,1}.
    {get_tl,{x,0},{x,0}}.
    {bif,self,{f,0},[],{x,1}}.
    {put_list,{x,1},nil,{x,1}}.
    {call_ext,2,{extfunc,erlang,'++',2}}.
    {move,{x,0},{x,1}}.
    {move,{y,1},{x,0}}.
    {init_yregs,{list,[{y,1}]}}.
    send.
  {label,3}.
    {loop_rec,{f,4},{x,0}}.
    remove_message.
    {move,{integer,0},{x,0}}.
    {line,[{location,"mandelbrot.erl",23}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},2}.
  {label,4}.
    {wait,{f,3}}.


{function, row, 6, 6}.
  {label,5}.
    {line,[{location,"mandelbrot.erl",26}]}.
    {func_info,{atom,mandelbrot},{atom,row},6}.
  {label,6}.
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,3},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,4},[{type,{t_list,{t_integer,{0,'+inf'}},nil}}]}}.
    {'%',{var_info,{x,5},[{type,{t_integer,any}}]}}.
    {test,is_eq_exact,
          {f,9},
          [{tr,{x,0},{t_integer,{0,'+inf'}}},{tr,{x,2},{t_integer,any}}]}.
    {allocate,4,6}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,5},{y,1}}.
    {move,{x,4},{y,2}}.
    {move,{x,3},{y,3}}.
  {label,7}.
    {loop_rec,{f,8},{x,0}}.
    {move,{x,0},{y,0}}.
    remove_message.
    {move,{y,1},{x,2}}.
    {move,{y,2},{x,1}}.
    {move,{y,3},{x,0}}.
    {move,{y,0},{y,3}}.
    {trim,3,1}.
    {line,[{location,"mandelbrot.erl",28}]}.
    {call,3,{f,18}}. % put_chars/3
    {line,[{location,"mandelbrot.erl",29}]}.
    {bif,hd,{f,0},[{y,0}],{x,0}}.
    {get_tl,{y,0},{x,1}}.
    send.
    {deallocate,1}.
    return.
  {label,8}.
    {wait,{f,7}}.
  {label,9}.
    {test,is_eq_exact,{f,10},[{tr,{x,5},{t_integer,any}},{integer,0}]}.
    {line,[{location,"mandelbrot.erl",33}]}.
    {gc_bif,'+',{f,0},5,[{tr,{x,0},{t_integer,{0,'+inf'}}},{integer,1}],{x,5}}.
    {gc_bif,'bsl',
            {f,0},
            6,
            [{tr,{x,3},{t_integer,{0,'+inf'}}},{integer,1}],
            {x,3}}.
    {gc_bif,'*',{f,0},6,[{tr,{x,0},{t_integer,{0,'+inf'}}},{integer,2}],{x,0}}.
    {fconv,{tr,{x,0},{t_integer,{0,'+inf'}}},{fr,0}}.
    {fconv,{tr,{x,2},{t_integer,any}},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,-1.5},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {allocate_heap,5,{alloc,[{words,0},{floats,1},{funs,0}]},6}.
    {move,{x,5},{y,0}}.
    {move,{x,3},{y,1}}.
    {move,{x,4},{y,2}}.
    {move,{x,2},{y,3}}.
    {move,{x,1},{y,4}}.
    {fmove,{fr,0},{x,1}}.
    {move,{y,4},{x,2}}.
    {move,{integer,50},{x,0}}.
    {call,3,{f,12}}. % m/3
    {'%',{var_info,{x,0},[{type,{t_integer,{0,1}}}]}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,1},{t_integer,{0,'+inf'}}},{tr,{x,0},{t_integer,{0,1}}}],
            {x,0}}.
    {test_heap,2,1}.
    {put_list,{x,0},{y,2},{x,4}}.
    {move,{y,3},{x,2}}.
    {move,{integer,0},{x,3}}.
    {move,{y,4},{x,1}}.
    {move,{integer,7},{x,5}}.
    {move,{y,0},{x,0}}.
    {call_last,6,{f,6},5}. % row/6
  {label,10}.
    {line,[{location,"mandelbrot.erl",36}]}.
    {gc_bif,'+',{f,0},6,[{tr,{x,0},{t_integer,{0,'+inf'}}},{integer,1}],{x,6}}.
    {gc_bif,'bsl',
            {f,0},
            7,
            [{tr,{x,3},{t_integer,{0,'+inf'}}},{integer,1}],
            {x,3}}.
    {gc_bif,'*',{f,0},7,[{tr,{x,0},{t_integer,{0,'+inf'}}},{integer,2}],{x,0}}.
    {fconv,{tr,{x,0},{t_integer,{0,'+inf'}}},{fr,0}}.
    {fconv,{tr,{x,2},{t_integer,any}},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,-1.5},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {allocate_heap,6,{alloc,[{words,0},{floats,1},{funs,0}]},7}.
    {move,{x,6},{y,0}}.
    {move,{x,3},{y,1}}.
    {move,{x,5},{y,2}}.
    {move,{x,4},{y,3}}.
    {move,{x,2},{y,4}}.
    {move,{x,1},{y,5}}.
    {fmove,{fr,0},{x,1}}.
    {move,{y,5},{x,2}}.
    {move,{integer,50},{x,0}}.
    {call,3,{f,12}}. % m/3
    {'%',{var_info,{x,0},[{type,{t_integer,{0,1}}}]}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,1},{t_integer,{0,'+inf'}}},{tr,{x,0},{t_integer,{0,1}}}],
            {x,0}}.
    {gc_bif,'-',{f,0},1,[{tr,{y,2},{t_integer,any}},{integer,1}],{x,5}}.
    {move,{y,4},{x,2}}.
    {move,{x,0},{x,3}}.
    {move,{y,3},{x,4}}.
    {move,{y,5},{x,1}}.
    {move,{y,0},{x,0}}.
    {call_last,6,{f,6},6}. % row/6


{function, m, 3, 12}.
  {label,11}.
    {line,[{location,"mandelbrot.erl",39}]}.
    {func_info,{atom,mandelbrot},{atom,m},3}.
  {label,12}.
    {'%',{var_info,{x,0},[{type,{t_integer,{50,50}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_float,any}}]}}.
    {move,{x,1},{x,3}}.
    {move,{x,2},{x,4}}.
    {move,{integer,49},{x,0}}.
    {call_only,5,{f,14}}. % m/5


{function, m, 5, 14}.
  {label,13}.
    {line,[{location,"mandelbrot.erl",41}]}.
    {func_info,{atom,mandelbrot},{atom,m},5}.
  {label,14}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,3},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,4},[{type,{t_float,any}}]}}.
    {fmove,{x,1},{fr,0}}.
    {line,[{location,"mandelbrot.erl",42}]}.
    {bif,fmul,{f,0},[{fr,0},{fr,0}],{fr,0}}.
    {fmove,{x,2},{fr,1}}.
    {bif,fmul,{f,0},[{fr,1},{fr,1}],{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,2}}.
    {test_heap,{alloc,[{words,0},{floats,3},{funs,0}]},5}.
    {fmove,{fr,2},{x,5}}.
    {fmove,{fr,0},{x,6}}.
    {fmove,{fr,1},{x,7}}.
    {test,is_lt,{f,15},[{float,4.0},{tr,{x,5},{t_float,any}}]}.
    {move,{integer,0},{x,0}}.
    return.
  {label,15}.
    {test,is_ge,{f,16},[{tr,{x,0},{t_integer,any}},{integer,1}]}.
    {line,[{location,"mandelbrot.erl",43}]}.
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
    {call_only,5,{f,14}}. % m/5
  {label,16}.
    {move,{integer,1},{x,0}}.
    return.


{function, put_chars, 3, 18}.
  {label,17}.
    {line,[{location,"mandelbrot.erl",48}]}.
    {func_info,{atom,mandelbrot},{atom,put_chars},3}.
  {label,18}.
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_list,{t_integer,{0,'+inf'}},nil}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {test,is_eq_exact,{f,19},[{tr,{x,2},{t_integer,any}},{integer,7}]}.
    {allocate,0,2}.
    {move,{x,1},{x,0}}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {call_ext_last,1,{extfunc,io,put_chars,1},0}.
  {label,19}.
    {line,[{location,"mandelbrot.erl",49}]}.
    {gc_bif,'+',{f,0},3,[{tr,{x,2},{t_integer,any}},{integer,1}],{x,2}}.
    {gc_bif,'bsl',
            {f,0},
            3,
            [{tr,{x,0},{t_integer,{0,'+inf'}}},{tr,{x,2},{t_integer,any}}],
            {x,0}}.
    {allocate_heap,0,2,2}.
    {put_list,{x,0},{x,1},{x,0}}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {call_ext_last,1,{extfunc,io,put_chars,1},0}.


{function, module_info, 0, 21}.
  {label,20}.
    {line,[]}.
    {func_info,{atom,mandelbrot},{atom,module_info},0}.
  {label,21}.
    {move,{atom,mandelbrot},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 23}.
  {label,22}.
    {line,[]}.
    {func_info,{atom,mandelbrot},{atom,module_info},1}.
  {label,23}.
    {move,{x,0},{x,1}}.
    {move,{atom,mandelbrot},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-main/1-fun-1-', 2, 25}.
  {label,24}.
    {line,[{location,"mandelbrot.erl",18}]}.
    {func_info,{atom,mandelbrot},{atom,'-main/1-fun-1-'},2}.
  {label,25}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {test_heap,{alloc,[{words,2},{floats,0},{funs,1}]},2}.
    {make_fun3,{f,27},0,0,{x,0},{list,[{x,0},{x,1}]}}.
    {call_ext_only,1,{extfunc,erlang,spawn,1}}.


{function, '-main/1-fun-0-', 2, 27}.
  {label,26}.
    {line,[{location,"mandelbrot.erl",18}]}.
    {func_info,{atom,mandelbrot},{atom,'-main/1-fun-0-'},2}.
  {label,27}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {gc_bif,'*',{f,0},2,[{x,0},{integer,2}],{x,0}}.
    {fconv,{tr,{x,0},{t_number,any}},{fr,0}}.
    {fconv,{tr,{x,1},{t_integer,any}},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,-1.0},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},2}.
    {fmove,{fr,0},{x,0}}.
    {move,{integer,0},{x,3}}.
    {move,{x,1},{x,2}}.
    {move,nil,{x,4}}.
    {move,{x,0},{x,1}}.
    {move,{integer,7},{x,5}}.
    {move,{integer,0},{x,0}}.
    {call_only,6,{f,6}}. % row/6
