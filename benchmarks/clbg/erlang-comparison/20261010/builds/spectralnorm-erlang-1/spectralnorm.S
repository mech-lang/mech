{module, spectralnorm}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 32}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"spectralnorm.erl",8}]}.
    {func_info,{atom,spectralnorm},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,1},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,1},[{x,2}]}.
    {allocate,2,2}.
    {init_yregs,{list,[{y,0},{y,1}]}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"spectralnorm.erl",9}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {move,{x,0},{y,1}}.
    {move,{x,0},{x,1}}.
    {move,nil,{x,2}}.
    {move,{float,1.0},{x,0}}.
    {line,[{location,"spectralnorm.erl",10}]}.
    {call,3,{f,26}}. % array/3
    {'%',{var_info,{x,0},[{type,{t_tuple,0,false,#{}}}]}}.
    {move,{x,0},{y,0}}.
    {move,{y,1},{x,1}}.
    {move,nil,{x,2}}.
    {move,{float,0.0},{x,0}}.
    {call,3,{f,26}}. % array/3
    {'%',{var_info,{x,0},[{type,{t_tuple,0,false,#{}}}]}}.
    {move,{y,0},{x,2}}.
    {move,{integer,10},{x,1}}.
    {move,{x,0},{x,3}}.
    {trim,1,1}.
    {move,{y,0},{x,0}}.
    {call,4,{f,7}}. % powerMethod/4
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,2,true,
                                   #{1 => {t_tuple,0,false,#{}},
                                     2 => {t_tuple,0,false,#{}}}}}]}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{x,2}}.
    {move,{float,0.0},{x,3}}.
    {move,{float,0.0},{x,4}}.
    {move,{y,0},{x,0}}.
    {trim,1,0}.
    {line,[{location,"spectralnorm.erl",11}]}.
    {call,5,{f,4}}. % loop/5
    {'%',{var_info,{x,0},[{type,{t_float,any}}]}}.
    {test_heap,2,1}.
    {put_list,{x,0},nil,{x,1}}.
    {move,{literal,"~.9f\n"},{x,0}}.
    {call_ext,2,{extfunc,io,format,2}}.
    {move,{integer,0},{x,0}}.
    {line,[{location,"spectralnorm.erl",12}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.


{function, loop, 5, 4}.
  {label,3}.
    {line,[{location,"spectralnorm.erl",15}]}.
    {func_info,{atom,spectralnorm},{atom,loop},5}.
  {label,4}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_tuple,0,false,#{}}}]}}.
    {'%',{var_info,{x,2},[{type,{t_tuple,0,false,#{}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,4},[{type,{t_float,any}}]}}.
    {test,is_eq_exact,{f,5},[{tr,{x,0},{t_integer,any}},{integer,0}]}.
    {fmove,{x,3},{fr,0}}.
    {fmove,{x,4},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},0}.
    {fmove,{fr,0},{x,0}}.
    {call_ext_only,1,{extfunc,math,sqrt,1}}.
  {label,5}.
    {line,[{location,"spectralnorm.erl",17}]}.
    {bif,element,
         {f,0},
         [{tr,{x,0},{t_integer,any}},{tr,{x,2},{t_tuple,0,false,#{}}}],
         {x,5}}.
    {line,[{location,"spectralnorm.erl",18}]}.
    {gc_bif,'-',
            {f,0},
            6,
            [{tr,{x,0},{t_integer,{1,16777215}}},{integer,1}],
            {x,6}}.
    {bif,element,
         {f,0},
         [{tr,{x,0},{t_integer,{1,16777215}}},
          {tr,{x,1},{t_tuple,0,false,#{}}}],
         {x,0}}.
    {gc_bif,'*',{f,0},7,[{x,0},{x,5}],{x,0}}.
    {fmove,{x,3},{fr,0}}.
    {fconv,{tr,{x,0},{t_number,any}},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},7}.
    {fmove,{fr,0},{x,0}}.
    {gc_bif,'*',
            {f,0},
            7,
            [{tr,{x,5},{t_number,any}},{tr,{x,5},{t_number,any}}],
            {x,3}}.
    {fmove,{x,4},{fr,0}}.
    {fconv,{tr,{x,3},{t_number,any}},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},7}.
    {fmove,{fr,0},{x,4}}.
    {move,{x,0},{x,3}}.
    {move,{x,6},{x,0}}.
    {call_only,5,{f,4}}. % loop/5


{function, powerMethod, 4, 7}.
  {label,6}.
    {line,[{location,"spectralnorm.erl",21}]}.
    {func_info,{atom,spectralnorm},{atom,powerMethod},4}.
  {label,7}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_tuple,0,false,#{}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_tuple,0,false,#{}}}]}}.
    {test,is_eq_exact,{f,8},[{tr,{x,1},{t_integer,any}},{integer,0}]}.
    {test_heap,3,4}.
    {put_tuple2,{x,0},{list,[{x,2},{x,3}]}}.
    return.
  {label,8}.
    {allocate,3,4}.
    {move,{x,2},{y,0}}.
    {move,{x,1},{y,1}}.
    {move,{x,0},{y,2}}.
    {move,{x,2},{x,1}}.
    {move,{x,3},{x,2}}.
    {line,[{location,"spectralnorm.erl",23}]}.
    {call,3,{f,24}}. % atav/3
    {'%',{var_info,{x,0},[{type,{t_tuple,0,false,#{}}}]}}.
    {move,{y,0},{x,2}}.
    {move,{y,2},{x,1}}.
    {move,{x,0},{y,0}}.
    {swap,{x,0},{x,1}}.
    {line,[{location,"spectralnorm.erl",24}]}.
    {call,3,{f,24}}. % atav/3
    {'%',{var_info,{x,0},[{type,{t_tuple,0,false,#{}}}]}}.
    {line,[{location,"spectralnorm.erl",25}]}.
    {gc_bif,'-',{f,0},1,[{tr,{y,1},{t_integer,any}},{integer,1}],{x,1}}.
    {move,{x,0},{x,2}}.
    {move,{y,0},{x,3}}.
    {move,{y,2},{x,0}}.
    {call_last,4,{f,7},3}. % powerMethod/4


{function, a, 2, 10}.
  {label,9}.
    {line,[{location,"spectralnorm.erl",28}]}.
    {func_info,{atom,spectralnorm},{atom,a},2}.
  {label,10}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {line,[{location,"spectralnorm.erl",29}]}.
    {fconv,{tr,{x,0},{t_integer,any}},{fr,0}}.
    {fmove,{float,1.0},{fr,1}}.
    {bif,fsub,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fconv,{tr,{x,1},{t_integer,any}},{fr,2}}.
    {bif,fsub,{f,0},[{fr,2},{fr,1}],{fr,2}}.
    {line,[{location,"spectralnorm.erl",30}]}.
    {bif,fadd,{f,0},[{fr,0},{fr,2}],{fr,2}}.
    {bif,fadd,{f,0},[{fr,2},{fr,1}],{fr,3}}.
    {bif,fmul,{f,0},[{fr,2},{fr,3}],{fr,2}}.
    {fmove,{float,2.0},{fr,3}}.
    {bif,fdiv,{f,0},[{fr,2},{fr,3}],{fr,2}}.
    {bif,fadd,{f,0},[{fr,2},{fr,0}],{fr,0}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {bif,fdiv,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},0}.
    {fmove,{fr,0},{x,0}}.
    return.


{function, av, 4, 12}.
  {label,11}.
    {line,[{location,"spectralnorm.erl",33}]}.
    {func_info,{atom,spectralnorm},{atom,av},4}.
  {label,12}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_tuple,0,false,#{}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_tuple,0,false,#{}}}]}}.
    {test,is_eq_exact,{f,13},[{tr,{x,1},{t_integer,any}},{integer,0}]}.
    {move,{x,3},{x,0}}.
    return.
  {label,13}.
    {line,[{location,"spectralnorm.erl",35}]}.
    {gc_bif,'-',{f,0},4,[{tr,{x,1},{t_integer,any}},{integer,1}],{x,4}}.
    {allocate,5,5}.
    {move,{x,4},{y,0}}.
    {move,{x,3},{y,1}}.
    {move,{x,2},{y,2}}.
    {move,{x,1},{y,3}}.
    {move,{x,0},{y,4}}.
    {move,{float,0.0},{x,3}}.
    {call,4,{f,15}}. % avloop/4
    {'%',{var_info,{x,0},[{type,{t_float,any}}]}}.
    {move,{y,1},{x,1}}.
    {move,{x,0},{x,2}}.
    {move,{y,3},{x,0}}.
    {init_yregs,{list,[{y,1},{y,3}]}}.
    {call_ext,3,{extfunc,erlang,setelement,3}}.
    {move,{y,0},{x,1}}.
    {move,{y,2},{x,2}}.
    {move,{x,0},{x,3}}.
    {move,{y,4},{x,0}}.
    {call_last,4,{f,12},5}. % av/4


{function, avloop, 4, 15}.
  {label,14}.
    {line,[{location,"spectralnorm.erl",37}]}.
    {func_info,{atom,spectralnorm},{atom,avloop},4}.
  {label,15}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_tuple,0,false,#{}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_float,any}}]}}.
    {test,is_eq_exact,{f,16},[{tr,{x,0},{t_integer,any}},{integer,0}]}.
    {move,{x,3},{x,0}}.
    return.
  {label,16}.
    {line,[{location,"spectralnorm.erl",39}]}.
    {gc_bif,'-',{f,0},4,[{tr,{x,0},{t_integer,any}},{integer,1}],{x,4}}.
    {allocate,5,5}.
    {move,{x,4},{y,0}}.
    {move,{x,3},{y,1}}.
    {move,{x,2},{y,2}}.
    {move,{x,1},{y,3}}.
    {move,{x,0},{y,4}}.
    {swap,{x,0},{x,1}}.
    {call,2,{f,10}}. % a/2
    {'%',{var_info,{x,0},[{type,{t_float,any}}]}}.
    {bif,element,
         {f,0},
         [{tr,{y,4},{t_integer,any}},{tr,{y,2},{t_tuple,0,false,#{}}}],
         {x,1}}.
    {fmove,{x,0},{fr,0}}.
    {fconv,{x,1},{fr,1}}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{y,1},{fr,1}}.
    {bif,fadd,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},0}.
    {fmove,{fr,0},{x,3}}.
    {move,{y,2},{x,2}}.
    {move,{y,3},{x,1}}.
    {move,{y,0},{x,0}}.
    {call_last,4,{f,15},5}. % avloop/4


{function, atv, 4, 18}.
  {label,17}.
    {line,[{location,"spectralnorm.erl",42}]}.
    {func_info,{atom,spectralnorm},{atom,atv},4}.
  {label,18}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_tuple,0,false,#{}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_tuple,0,false,#{}}}]}}.
    {test,is_eq_exact,{f,19},[{tr,{x,1},{t_integer,any}},{integer,0}]}.
    {move,{x,3},{x,0}}.
    return.
  {label,19}.
    {line,[{location,"spectralnorm.erl",44}]}.
    {gc_bif,'-',{f,0},4,[{tr,{x,1},{t_integer,any}},{integer,1}],{x,4}}.
    {allocate,5,5}.
    {move,{x,4},{y,0}}.
    {move,{x,3},{y,1}}.
    {move,{x,2},{y,2}}.
    {move,{x,1},{y,3}}.
    {move,{x,0},{y,4}}.
    {move,{float,0.0},{x,3}}.
    {call,4,{f,21}}. % atvloop/4
    {'%',{var_info,{x,0},[{type,{t_float,any}}]}}.
    {move,{y,1},{x,1}}.
    {move,{x,0},{x,2}}.
    {move,{y,3},{x,0}}.
    {init_yregs,{list,[{y,1},{y,3}]}}.
    {call_ext,3,{extfunc,erlang,setelement,3}}.
    {move,{y,0},{x,1}}.
    {move,{y,2},{x,2}}.
    {move,{x,0},{x,3}}.
    {move,{y,4},{x,0}}.
    {call_last,4,{f,18},5}. % atv/4


{function, atvloop, 4, 21}.
  {label,20}.
    {line,[{location,"spectralnorm.erl",46}]}.
    {func_info,{atom,spectralnorm},{atom,atvloop},4}.
  {label,21}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_tuple,0,false,#{}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_float,any}}]}}.
    {test,is_eq_exact,{f,22},[{tr,{x,0},{t_integer,any}},{integer,0}]}.
    {move,{x,3},{x,0}}.
    return.
  {label,22}.
    {line,[{location,"spectralnorm.erl",47}]}.
    {gc_bif,'-',{f,0},4,[{tr,{x,0},{t_integer,any}},{integer,1}],{x,4}}.
    {allocate,5,5}.
    {move,{x,4},{y,0}}.
    {move,{x,3},{y,1}}.
    {move,{x,2},{y,2}}.
    {move,{x,1},{y,3}}.
    {move,{x,0},{y,4}}.
    {call,2,{f,10}}. % a/2
    {'%',{var_info,{x,0},[{type,{t_float,any}}]}}.
    {bif,element,
         {f,0},
         [{tr,{y,4},{t_integer,any}},{tr,{y,2},{t_tuple,0,false,#{}}}],
         {x,1}}.
    {fmove,{x,0},{fr,0}}.
    {fconv,{x,1},{fr,1}}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{y,1},{fr,1}}.
    {bif,fadd,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},0}.
    {fmove,{fr,0},{x,3}}.
    {move,{y,2},{x,2}}.
    {move,{y,3},{x,1}}.
    {move,{y,0},{x,0}}.
    {call_last,4,{f,21},5}. % atvloop/4


{function, atav, 3, 24}.
  {label,23}.
    {line,[{location,"spectralnorm.erl",50}]}.
    {func_info,{atom,spectralnorm},{atom,atav},3}.
  {label,24}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_tuple,0,false,#{}}}]}}.
    {'%',{var_info,{x,2},[{type,{t_tuple,0,false,#{}}}]}}.
    {allocate,3,3}.
    {move,{x,2},{y,0}}.
    {move,{x,1},{y,1}}.
    {move,{x,0},{y,2}}.
    {move,{x,0},{x,1}}.
    {move,nil,{x,2}}.
    {move,{float,0.0},{x,0}}.
    {line,[{location,"spectralnorm.erl",51}]}.
    {call,3,{f,26}}. % array/3
    {'%',{var_info,{x,0},[{type,{t_tuple,0,false,#{}}}]}}.
    {move,{y,1},{x,2}}.
    {move,{y,2},{x,1}}.
    {move,{x,0},{x,3}}.
    {move,{y,0},{y,1}}.
    {trim,1,2}.
    {move,{y,1},{x,0}}.
    {call,4,{f,12}}. % av/4
    {'%',{var_info,{x,0},[{type,{t_tuple,0,false,#{}}}]}}.
    {move,{x,0},{x,2}}.
    {move,{y,0},{x,3}}.
    {move,{y,1},{x,1}}.
    {move,{y,1},{x,0}}.
    {call_last,4,{f,18},2}. % atv/4


{function, array, 3, 26}.
  {label,25}.
    {line,[{location,"spectralnorm.erl",54}]}.
    {func_info,{atom,spectralnorm},{atom,array},3}.
  {label,26}.
    {'%',{var_info,{x,0},[{type,{t_float,{0.0,1.0}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_list,{t_float,{0.0,1.0}},nil}}]}}.
    {test,is_eq_exact,{f,27},[{tr,{x,1},{t_integer,any}},{integer,0}]}.
    {move,{x,2},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,list_to_tuple,1}}.
  {label,27}.
    {line,[{location,"spectralnorm.erl",55}]}.
    {gc_bif,'-',{f,0},3,[{tr,{x,1},{t_integer,any}},{integer,1}],{x,1}}.
    {test_heap,2,3}.
    {put_list,{x,0},{x,2},{x,2}}.
    {call_only,3,{f,26}}. % array/3


{function, module_info, 0, 29}.
  {label,28}.
    {line,[]}.
    {func_info,{atom,spectralnorm},{atom,module_info},0}.
  {label,29}.
    {move,{atom,spectralnorm},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 31}.
  {label,30}.
    {line,[]}.
    {func_info,{atom,spectralnorm},{atom,module_info},1}.
  {label,31}.
    {move,{x,0},{x,1}}.
    {move,{atom,spectralnorm},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.
