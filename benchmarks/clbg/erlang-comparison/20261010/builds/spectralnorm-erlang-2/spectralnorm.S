{module, spectralnorm}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 67}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"spectralnorm.erl",9}]}.
    {func_info,{atom,spectralnorm},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,1},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,1},[{x,2}]}.
    {allocate,1,2}.
    {move,{x,1},{y,0}}.
    {bif,self,{f,0},[],{x,1}}.
    {move,{atom,server},{x,0}}.
    {line,[{location,"spectralnorm.erl",10}]}.
    {call_ext,2,{extfunc,erlang,register,2}}.
    {move,{y,0},{x,0}}.
    {init_yregs,{list,[{y,0}]}}.
    {line,[{location,"spectralnorm.erl",11}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {move,{x,0},{y,0}}.
    {move,{integer,1},{x,1}}.
    {line,[{location,"spectralnorm.erl",12}]}.
    {call_ext,2,{extfunc,erlang,make_tuple,2}}.
    {move,{x,0},{x,2}}.
    {move,{integer,10},{x,1}}.
    {move,nil,{x,3}}.
    {move,{y,0},{x,0}}.
    {call,4,{f,8}}. % power_method/4
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_tuple,2,true,
                     #{2 =>
                           {t_union,none,nil,none,
                               {t_tuple,0,false,#{}},
                               none,none}}}}]}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{x,2}}.
    {move,{integer,0},{x,3}}.
    {move,{integer,0},{x,4}}.
    {move,{y,0},{x,0}}.
    {trim,1,0}.
    {line,[{location,"spectralnorm.erl",13}]}.
    {call,5,{f,4}}. % eigen/5
    {'%',{var_info,{x,0},[{type,{t_float,any}}]}}.
    {test_heap,2,1}.
    {put_list,{x,0},nil,{x,1}}.
    {move,{literal,"~.9f\n"},{x,0}}.
    {call_ext,2,{extfunc,io,format,2}}.
    {move,{integer,0},{x,0}}.
    {line,[{location,"spectralnorm.erl",14}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.


{function, eigen, 5, 4}.
  {label,3}.
    {line,[{location,"spectralnorm.erl",17}]}.
    {func_info,{atom,spectralnorm},{atom,eigen},5}.
  {label,4}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},
                   [{type,{t_union,none,nil,none,
                                   {t_tuple,0,false,#{}},
                                   none,none}}]}}.
    {'%',{var_info,{x,3},[{type,{t_number,any}}]}}.
    {'%',{var_info,{x,4},[{type,{t_number,any}}]}}.
    {test,is_eq_exact,{f,6},[{tr,{x,0},{t_integer,any}},{integer,0}]}.
    {test,is_eq,{f,5},[{tr,{x,4},{t_number,any}},{integer,0}]}.
    {move,{integer,0},{x,0}}.
    {jump,{f,3}}.
  {label,5}.
    {fconv,{tr,{x,3},{t_number,any}},{fr,0}}.
    {fconv,{tr,{x,4},{t_number,any}},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},0}.
    {fmove,{fr,0},{x,0}}.
    {call_ext_only,1,{extfunc,math,sqrt,1}}.
  {label,6}.
    {line,[{location,"spectralnorm.erl",20}]}.
    {bif,element,
         {f,0},
         [{tr,{x,0},{t_integer,any}},
          {tr,{x,2},{t_union,none,nil,none,{t_tuple,0,false,#{}},none,none}}],
         {x,5}}.
    {line,[{location,"spectralnorm.erl",21}]}.
    {gc_bif,'-',
            {f,0},
            6,
            [{tr,{x,0},{t_integer,{1,16777215}}},{integer,1}],
            {x,6}}.
    {bif,element,{f,0},[{tr,{x,0},{t_integer,{1,16777215}}},{x,1}],{x,0}}.
    {gc_bif,'*',{f,0},7,[{x,0},{x,5}],{x,0}}.
    {gc_bif,'+',
            {f,0},
            7,
            [{tr,{x,3},{t_number,any}},{tr,{x,0},{t_number,any}}],
            {x,0}}.
    {gc_bif,'*',
            {f,0},
            7,
            [{tr,{x,5},{t_number,any}},{tr,{x,5},{t_number,any}}],
            {x,3}}.
    {gc_bif,'+',
            {f,0},
            7,
            [{tr,{x,4},{t_number,any}},{tr,{x,3},{t_number,any}}],
            {x,4}}.
    {move,{x,0},{x,3}}.
    {move,{x,6},{x,0}}.
    {call_only,5,{f,4}}. % eigen/5


{function, power_method, 4, 8}.
  {label,7}.
    {line,[{location,"spectralnorm.erl",24}]}.
    {func_info,{atom,spectralnorm},{atom,power_method},4}.
  {label,8}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,3},
                   [{type,{t_union,none,nil,none,
                                   {t_tuple,0,false,#{}},
                                   none,none}}]}}.
    {test,is_eq_exact,{f,9},[{tr,{x,1},{t_integer,any}},{integer,0}]}.
    {test_heap,3,4}.
    {put_tuple2,{x,0},{list,[{x,2},{x,3}]}}.
    return.
  {label,9}.
    {allocate,3,3}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,1},{y,1}}.
    {move,{x,0},{y,2}}.
    {move,{x,2},{x,1}}.
    {line,[{location,"spectralnorm.erl",26}]}.
    {call,2,{f,25}}. % atav/2
    {'%',{var_info,{x,0},[{type,{t_tuple,0,false,#{}}}]}}.
    {move,{x,0},{y,0}}.
    {move,{x,0},{x,1}}.
    {move,{y,2},{x,0}}.
    {line,[{location,"spectralnorm.erl",27}]}.
    {call,2,{f,25}}. % atav/2
    {'%',{var_info,{x,0},[{type,{t_tuple,0,false,#{}}}]}}.
    {line,[{location,"spectralnorm.erl",28}]}.
    {gc_bif,'-',{f,0},1,[{tr,{y,1},{t_integer,any}},{integer,1}],{x,1}}.
    {move,{x,0},{x,2}}.
    {move,{y,0},{x,3}}.
    {move,{y,2},{x,0}}.
    {call_last,4,{f,8},3}. % power_method/4


{function, av, 2, 11}.
  {label,10}.
    {line,[{location,"spectralnorm.erl",36}]}.
    {func_info,{atom,spectralnorm},{atom,av},2}.
  {label,11}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {test_heap,{alloc,[{words,2},{floats,0},{funs,1}]},2}.
    {make_fun3,{f,66},0,0,{x,1},{list,[{x,0},{x,1}]}}.
    {call_only,2,{f,27}}. % pmap/2


{function, av, 4, 13}.
  {label,12}.
    {line,[{location,"spectralnorm.erl",38}]}.
    {func_info,{atom,spectralnorm},{atom,av},4}.
  {label,13}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {allocate,3,4}.
    {move,{x,3},{y,1}}.
    {move,{x,0},{y,2}}.
    {bif,self,{f,0},[],{y,0}}.
    {move,{x,1},{x,0}}.
    {move,{x,2},{x,1}}.
    {call_ext,2,{extfunc,lists,seq,2}}.
    {move,{y,1},{x,2}}.
    {move,{y,2},{x,1}}.
    {move,{y,0},{y,2}}.
    {trim,2,1}.
    {call,3,{f,60}}. % '-av/4-lc$^0/1-0-'/3
    {'%',{var_info,{x,0},[{type,{t_list,{t_float,any},nil}}]}}.
    {test_heap,3,1}.
    {put_tuple2,{x,1},{list,[{y,0},{x,0}]}}.
    {move,{atom,server},{x,0}}.
    send.
    {deallocate,1}.
    return.


{function, avloop, 4, 15}.
  {label,14}.
    {line,[{location,"spectralnorm.erl",40}]}.
    {func_info,{atom,spectralnorm},{atom,avloop},4}.
  {label,15}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_number,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_tuple,0,false,#{}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_float,any}}]}}.
    {test,is_eq_exact,{f,16},[{tr,{x,0},{t_integer,any}},{integer,0}]}.
    {move,{x,3},{x,0}}.
    return.
  {label,16}.
    {line,[{location,"spectralnorm.erl",41}]}.
    {gc_bif,'-',{f,0},4,[{tr,{x,0},{t_integer,any}},{integer,1}],{x,4}}.
    {line,[{location,"spectralnorm.erl",32}]}.
    {gc_bif,'+',
            {f,0},
            5,
            [{tr,{x,1},{t_number,any}},{tr,{x,0},{t_integer,any}}],
            {x,5}}.
    {gc_bif,'-',{f,0},6,[{tr,{x,5},{t_number,any}},{integer,2}],{x,5}}.
    {gc_bif,'-',{f,0},6,[{tr,{x,1},{t_number,any}},{integer,1}],{x,6}}.
    {gc_bif,'+',
            {f,0},
            7,
            [{tr,{x,6},{t_number,any}},{tr,{x,0},{t_integer,any}}],
            {x,6}}.
    {gc_bif,'*',
            {f,0},
            7,
            [{tr,{x,5},{t_number,any}},{tr,{x,6},{t_number,any}}],
            {x,5}}.
    {fconv,{tr,{x,5},{t_number,any}},{fr,0}}.
    {fmove,{float,2.0},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fconv,{tr,{x,1},{t_number,any}},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,1.0},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},5}.
    {fmove,{fr,0},{x,5}}.
    {line,[{location,"spectralnorm.erl",41}]}.
    {bif,element,
         {f,0},
         [{tr,{x,0},{t_integer,any}},{tr,{x,2},{t_tuple,0,false,#{}}}],
         {x,0}}.
    {fmove,{x,5},{fr,0}}.
    {fconv,{x,0},{fr,1}}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{x,3},{fr,1}}.
    {bif,fadd,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},5}.
    {fmove,{fr,0},{x,3}}.
    {move,{x,4},{x,0}}.
    {call_only,4,{f,15}}. % avloop/4


{function, atv, 2, 18}.
  {label,17}.
    {line,[{location,"spectralnorm.erl",45}]}.
    {func_info,{atom,spectralnorm},{atom,atv},2}.
  {label,18}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_tuple,0,false,#{}}}]}}.
    {test_heap,{alloc,[{words,2},{floats,0},{funs,1}]},2}.
    {make_fun3,{f,58},0,0,{x,1},{list,[{x,0},{x,1}]}}.
    {call_only,2,{f,27}}. % pmap/2


{function, atv, 4, 20}.
  {label,19}.
    {line,[{location,"spectralnorm.erl",47}]}.
    {func_info,{atom,spectralnorm},{atom,atv},4}.
  {label,20}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,3},[{type,{t_tuple,0,false,#{}}}]}}.
    {allocate,3,4}.
    {move,{x,3},{y,1}}.
    {move,{x,0},{y,2}}.
    {bif,self,{f,0},[],{y,0}}.
    {move,{x,1},{x,0}}.
    {move,{x,2},{x,1}}.
    {call_ext,2,{extfunc,lists,seq,2}}.
    {move,{y,1},{x,2}}.
    {move,{y,2},{x,1}}.
    {move,{y,0},{y,2}}.
    {trim,2,1}.
    {call,3,{f,52}}. % '-atv/4-lc$^0/1-0-'/3
    {'%',{var_info,{x,0},[{type,{t_list,{t_float,any},nil}}]}}.
    {test_heap,3,1}.
    {put_tuple2,{x,1},{list,[{y,0},{x,0}]}}.
    {move,{atom,server},{x,0}}.
    send.
    {deallocate,1}.
    return.


{function, atvloop, 4, 22}.
  {label,21}.
    {line,[{location,"spectralnorm.erl",49}]}.
    {func_info,{atom,spectralnorm},{atom,atvloop},4}.
  {label,22}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_number,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_tuple,0,false,#{}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_float,any}}]}}.
    {test,is_eq_exact,{f,23},[{tr,{x,0},{t_integer,any}},{integer,0}]}.
    {move,{x,3},{x,0}}.
    return.
  {label,23}.
    {line,[{location,"spectralnorm.erl",50}]}.
    {gc_bif,'-',{f,0},4,[{tr,{x,0},{t_integer,any}},{integer,1}],{x,4}}.
    {line,[{location,"spectralnorm.erl",32}]}.
    {gc_bif,'+',
            {f,0},
            5,
            [{tr,{x,0},{t_integer,any}},{tr,{x,1},{t_number,any}}],
            {x,5}}.
    {gc_bif,'-',{f,0},6,[{tr,{x,5},{t_number,any}},{integer,2}],{x,5}}.
    {gc_bif,'+',
            {f,0},
            6,
            [{tr,{x,4},{t_integer,any}},{tr,{x,1},{t_number,any}}],
            {x,6}}.
    {gc_bif,'*',
            {f,0},
            7,
            [{tr,{x,5},{t_number,any}},{tr,{x,6},{t_number,any}}],
            {x,5}}.
    {fconv,{tr,{x,5},{t_number,any}},{fr,0}}.
    {fmove,{float,2.0},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fconv,{tr,{x,0},{t_integer,any}},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,1.0},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},5}.
    {fmove,{fr,0},{x,5}}.
    {line,[{location,"spectralnorm.erl",50}]}.
    {bif,element,
         {f,0},
         [{tr,{x,0},{t_integer,any}},{tr,{x,2},{t_tuple,0,false,#{}}}],
         {x,0}}.
    {fmove,{x,5},{fr,0}}.
    {fconv,{x,0},{fr,1}}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{x,3},{fr,1}}.
    {bif,fadd,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},5}.
    {fmove,{fr,0},{x,3}}.
    {move,{x,4},{x,0}}.
    {call_only,4,{f,22}}. % atvloop/4


{function, atav, 2, 25}.
  {label,24}.
    {line,[{location,"spectralnorm.erl",54}]}.
    {func_info,{atom,spectralnorm},{atom,atav},2}.
  {label,25}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {allocate,1,2}.
    {move,{x,0},{y,0}}.
    {call,2,{f,11}}. % av/2
    {'%',{var_info,{x,0},[{type,{t_tuple,0,false,#{}}}]}}.
    {move,{x,0},{x,1}}.
    {move,{y,0},{x,0}}.
    {call_last,2,{f,18},1}. % atv/2


{function, pmap, 2, 27}.
  {label,26}.
    {line,[{location,"spectralnorm.erl",58}]}.
    {func_info,{atom,spectralnorm},{atom,pmap},2}.
  {label,27}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_fun,2,any,any}}]}}.
    {allocate,2,2}.
    {move,{x,1},{y,0}}.
    {move,{x,0},{y,1}}.
    {move,{atom,logical_processors},{x,0}}.
    {line,[{location,"spectralnorm.erl",59}]}.
    {call_ext,1,{extfunc,erlang,system_info,1}}.
    {move,{y,1},{x,2}}.
    {move,{x,0},{x,1}}.
    {move,nil,{x,3}}.
    {init_yregs,{list,[{y,1}]}}.
    {move,{integer,0},{x,0}}.
    {call,4,{f,32}}. % chunks/4
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,any},2 => {t_integer,any}}},
                     nil}}]}}.
    {test_heap,{alloc,[{words,3},{floats,0},{funs,1}]},1}.
    {get_list,{x,0},{x,0},{y,1}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{x,0}}.
    {make_fun3,{f,45},0,0,{x,0},{list,[{y,0},{x,0},{x,1}]}}.
    {line,[{location,"spectralnorm.erl",60}]}.
    {call_ext,1,{extfunc,erlang,spawn,1}}.
    {swap,{y,1},{x,0}}.
    {move,{y,0},{x,1}}.
    {init_yregs,{list,[{y,0}]}}.
    {call,2,{f,47}}. % '-pmap/2-lc$^0/1-0-'/2
    {'%',{var_info,{x,0},[{type,{t_list,pid,nil}}]}}.
    {move,{x,0},{y,0}}.
  {label,28}.
    {loop_rec,{f,30},{x,0}}.
    {test,is_tuple,{f,29},[{x,0}]}.
    {test,test_arity,{f,29},[{x,0},2]}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {test,is_eq_exact,{f,29},[{x,1},{tr,{y,1},pid}]}.
    {move,{x,0},{y,1}}.
    remove_message.
    {move,{y,0},{x,0}}.
    {trim,1,1}.
    {line,[{location,"spectralnorm.erl",61}]}.
    {call,1,{f,39}}. % '-pmap/2-lc$^2/1-1-'/1
    {'%',{var_info,{x,0},[{type,{t_list,any,nil}}]}}.
    {test_heap,2,1}.
    {get_tuple_element,{y,0},1,{x,1}}.
    {put_list,{x,1},{x,0},{x,0}}.
    {trim,1,0}.
    {line,[{location,"spectralnorm.erl",62}]}.
    {call_ext,1,{extfunc,lists,flatten,1}}.
    {call_ext_last,1,{extfunc,erlang,list_to_tuple,1},0}.
  {label,29}.
    {loop_rec_end,{f,28}}.
  {label,30}.
    {wait,{f,28}}.


{function, chunks, 4, 32}.
  {label,31}.
    {line,[{location,"spectralnorm.erl",64}]}.
    {func_info,{atom,spectralnorm},{atom,chunks},4}.
  {label,32}.
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {'%',
        {var_info,
            {x,3},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => {t_integer,any},2 => {t_integer,any}}},
                     nil}}]}}.
    {gc_bif,'-',{f,33},4,[{x,1},{integer,1}],{x,4}}.
    {test,is_eq,
          {f,33},
          [{tr,{x,0},{t_integer,{0,'+inf'}}},{tr,{x,4},{t_number,any}}]}.
    {gc_bif,'div',
            {f,0},
            4,
            [{tr,{x,2},{t_integer,any}},{tr,{x,1},{t_number,any}}],
            {x,1}}.
    {gc_bif,'*',
            {f,0},
            4,
            [{tr,{x,0},{t_integer,{0,'+inf'}}},{tr,{x,1},{t_integer,any}}],
            {x,0}}.
    {gc_bif,'+',{f,0},4,[{tr,{x,0},{t_integer,any}},{integer,1}],{x,0}}.
    {test_heap,5,4}.
    {put_tuple2,{x,0},{list,[{x,0},{x,2}]}}.
    {put_list,{x,0},{x,3},{x,0}}.
    {call_ext_only,1,{extfunc,lists,reverse,1}}.
  {label,33}.
    {line,[{location,"spectralnorm.erl",65}]}.
    {gc_bif,'+',{f,0},4,[{tr,{x,0},{t_integer,{0,'+inf'}}},{integer,1}],{x,4}}.
    {gc_bif,'div',{f,0},5,[{tr,{x,2},{t_integer,any}},{x,1}],{x,5}}.
    {gc_bif,'*',
            {f,0},
            6,
            [{tr,{x,0},{t_integer,{0,'+inf'}}},{tr,{x,5},{t_integer,any}}],
            {x,0}}.
    {gc_bif,'+',{f,0},6,[{tr,{x,0},{t_integer,any}},{integer,1}],{x,0}}.
    {gc_bif,'*',
            {f,0},
            6,
            [{tr,{x,4},{t_integer,{1,'+inf'}}},{tr,{x,5},{t_integer,any}}],
            {x,5}}.
    {test_heap,5,6}.
    {put_tuple2,{x,0},{list,[{x,0},{x,5}]}}.
    {put_list,{x,0},{x,3},{x,3}}.
    {move,{x,4},{x,0}}.
    {call_only,4,{f,32}}. % chunks/4


{function, module_info, 0, 35}.
  {label,34}.
    {line,[]}.
    {func_info,{atom,spectralnorm},{atom,module_info},0}.
  {label,35}.
    {move,{atom,spectralnorm},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 37}.
  {label,36}.
    {line,[]}.
    {func_info,{atom,spectralnorm},{atom,module_info},1}.
  {label,37}.
    {move,{x,0},{x,1}}.
    {move,{atom,spectralnorm},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-pmap/2-lc$^2/1-1-', 1, 39}.
  {label,38}.
    {line,[{location,"spectralnorm.erl",61}]}.
    {func_info,{atom,spectralnorm},{atom,'-pmap/2-lc$^2/1-1-'},1}.
  {label,39}.
    {'%',{var_info,{x,0},[{type,{t_list,pid,nil}}]}}.
    {test,is_nonempty_list,{f,43},[{x,0}]}.
    {allocate,2,1}.
    {get_list,{x,0},{y,1},{y,0}}.
  {label,40}.
    {loop_rec,{f,42},{x,0}}.
    {test,is_tuple,{f,41},[{x,0}]}.
    {test,test_arity,{f,41},[{x,0},2]}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {test,is_eq_exact,{f,41},[{x,1},{tr,{y,1},pid}]}.
    {move,{x,0},{y,1}}.
    remove_message.
    {move,{y,0},{x,0}}.
    {trim,1,1}.
    {call,1,{f,39}}. % '-pmap/2-lc$^2/1-1-'/1
    {'%',{var_info,{x,0},[{type,{t_list,any,nil}}]}}.
    {test_heap,2,1}.
    {get_tuple_element,{y,0},1,{x,1}}.
    {put_list,{x,1},{x,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,41}.
    {loop_rec_end,{f,40}}.
  {label,42}.
    {wait,{f,40}}.
  {label,43}.
    return.


{function, '-pmap/2-anonymous-3-', 3, 45}.
  {label,44}.
    {line,[{location,"spectralnorm.erl",60}]}.
    {func_info,{atom,spectralnorm},{atom,'-pmap/2-anonymous-3-'},3}.
  {label,45}.
    {'%',{var_info,{x,0},[{type,{t_fun,2,any,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {allocate,0,3}.
    {swap,{x,0},{x,2}}.
    {call_fun2,{atom,safe},2,{tr,{x,2},{t_fun,2,any,any}}}.
    {deallocate,0}.
    return.


{function, '-pmap/2-lc$^0/1-0-', 2, 47}.
  {label,46}.
    {line,[{location,"spectralnorm.erl",60}]}.
    {func_info,{atom,spectralnorm},{atom,'-pmap/2-lc$^0/1-0-'},2}.
  {label,47}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => {t_integer,any},2 => {t_integer,any}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_fun,2,any,any}}]}}.
    {test,is_nonempty_list,{f,48},[{x,0}]}.
    {allocate_heap,2,{alloc,[{words,3},{floats,0},{funs,1}]},2}.
    {move,{x,1},{y,1}}.
    {get_list,{x,0},{x,0},{y,0}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{x,0}}.
    {make_fun3,{f,50},0,0,{x,0},{list,[{y,1},{x,0},{x,1}]}}.
    {call_ext,1,{extfunc,erlang,spawn,1}}.
    {move,{y,0},{x,2}}.
    {move,{y,1},{x,1}}.
    {move,{x,0},{y,1}}.
    {trim,1,1}.
    {move,{x,2},{x,0}}.
    {call,2,{f,47}}. % '-pmap/2-lc$^0/1-0-'/2
    {'%',{var_info,{x,0},[{type,{t_list,pid,nil}}]}}.
    {test_heap,2,1}.
    {put_list,{y,0},{x,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,48}.
    return.


{function, '-pmap/2-anonymous-2-', 3, 50}.
  {label,49}.
    {line,[{location,"spectralnorm.erl",60}]}.
    {func_info,{atom,spectralnorm},{atom,'-pmap/2-anonymous-2-'},3}.
  {label,50}.
    {'%',{var_info,{x,0},[{type,{t_fun,2,any,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {allocate,0,3}.
    {swap,{x,0},{x,2}}.
    {call_fun2,{atom,safe},2,{tr,{x,2},{t_fun,2,any,any}}}.
    {deallocate,0}.
    return.


{function, '-atv/4-lc$^0/1-0-', 3, 52}.
  {label,51}.
    {line,[{location,"spectralnorm.erl",47}]}.
    {func_info,{atom,spectralnorm},{atom,'-atv/4-lc$^0/1-0-'},3}.
  {label,52}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_tuple,0,false,#{}}}]}}.
    {test,is_nonempty_list,{f,55},[{x,0}]}.
    {allocate,4,3}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,2},{y,2}}.
    {move,{x,1},{y,3}}.
    {get_list,{x,0},{x,0},{y,1}}.
    {test,is_eq_exact,{f,53},[{tr,{x,1},{t_integer,any}},{integer,0}]}.
    {move,{float,0.0},{y,0}}.
    {jump,{f,54}}.
  {label,53}.
    {line,[{location,"spectralnorm.erl",50}]}.
    {gc_bif,'-',{f,0},2,[{tr,{x,1},{t_integer,any}},{integer,1}],{x,1}}.
    {line,[{location,"spectralnorm.erl",32}]}.
    {gc_bif,'+',{f,0},2,[{tr,{y,3},{t_integer,any}},{x,0}],{x,2}}.
    {gc_bif,'-',{f,0},3,[{tr,{x,2},{t_number,any}},{integer,2}],{x,2}}.
    {gc_bif,'+',
            {f,0},
            3,
            [{tr,{x,1},{t_integer,any}},{tr,{x,0},{t_number,any}}],
            {x,3}}.
    {gc_bif,'*',
            {f,0},
            4,
            [{tr,{x,2},{t_number,any}},{tr,{x,3},{t_number,any}}],
            {x,2}}.
    {fconv,{tr,{x,2},{t_number,any}},{fr,0}}.
    {fmove,{float,2.0},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fconv,{tr,{y,3},{t_integer,any}},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,1.0},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},2}.
    {fmove,{fr,0},{x,2}}.
    {line,[{location,"spectralnorm.erl",50}]}.
    {bif,element,
         {f,0},
         [{tr,{y,3},{t_integer,any}},{tr,{y,2},{t_tuple,0,false,#{}}}],
         {x,3}}.
    {fmove,{x,2},{fr,0}}.
    {fconv,{x,3},{fr,1}}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,0.0},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},2}.
    {fmove,{fr,0},{x,3}}.
    {move,{y,2},{x,2}}.
    {swap,{x,0},{x,1}}.
    {call,4,{f,22}}. % atvloop/4
    {'%',{var_info,{x,0},[{type,{t_float,any}}]}}.
    {move,{x,0},{y,0}}.
  {label,54}.
    {move,{y,2},{x,2}}.
    {move,{y,3},{x,1}}.
    {move,{y,1},{x,0}}.
    {move,{y,0},{y,3}}.
    {trim,3,1}.
    {line,[{location,"spectralnorm.erl",47}]}.
    {call,3,{f,52}}. % '-atv/4-lc$^0/1-0-'/3
    {'%',{var_info,{x,0},[{type,{t_list,{t_float,any},nil}}]}}.
    {test_heap,2,1}.
    {put_list,{y,0},{x,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,55}.
    {test,is_nil,{f,56},[{x,0}]}.
    return.
  {label,56}.
    {test_heap,3,1}.
    {put_tuple2,{x,0},{list,[{atom,bad_generator},{x,0}]}}.
    {call_ext_only,1,{extfunc,erlang,error,1}}.


{function, '-atv/2-anonymous-0-', 4, 58}.
  {label,57}.
    {line,[{location,"spectralnorm.erl",45}]}.
    {func_info,{atom,spectralnorm},{atom,'-atv/2-anonymous-0-'},4}.
  {label,58}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,3},[{type,{t_tuple,0,false,#{}}}]}}.
    {swap,{x,1},{x,2}}.
    {swap,{x,0},{x,1}}.
    {call_only,4,{f,20}}. % atv/4


{function, '-av/4-lc$^0/1-0-', 3, 60}.
  {label,59}.
    {line,[{location,"spectralnorm.erl",38}]}.
    {func_info,{atom,spectralnorm},{atom,'-av/4-lc$^0/1-0-'},3}.
  {label,60}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {test,is_nonempty_list,{f,63},[{x,0}]}.
    {allocate,4,3}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,2},{y,2}}.
    {move,{x,1},{y,3}}.
    {get_list,{x,0},{x,0},{y,1}}.
    {test,is_eq_exact,{f,61},[{tr,{x,1},{t_integer,any}},{integer,0}]}.
    {move,{float,0.0},{y,0}}.
    {jump,{f,62}}.
  {label,61}.
    {line,[{location,"spectralnorm.erl",41}]}.
    {gc_bif,'-',{f,0},2,[{tr,{x,1},{t_integer,any}},{integer,1}],{x,1}}.
    {line,[{location,"spectralnorm.erl",32}]}.
    {gc_bif,'+',{f,0},2,[{x,0},{tr,{y,3},{t_integer,any}}],{x,2}}.
    {gc_bif,'-',{f,0},3,[{tr,{x,2},{t_number,any}},{integer,2}],{x,2}}.
    {gc_bif,'-',{f,0},3,[{tr,{x,0},{t_number,any}},{integer,1}],{x,3}}.
    {gc_bif,'+',
            {f,0},
            4,
            [{tr,{x,3},{t_number,any}},{tr,{y,3},{t_integer,any}}],
            {x,3}}.
    {gc_bif,'*',
            {f,0},
            4,
            [{tr,{x,2},{t_number,any}},{tr,{x,3},{t_number,any}}],
            {x,2}}.
    {fconv,{tr,{x,2},{t_number,any}},{fr,0}}.
    {fmove,{float,2.0},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fconv,{tr,{x,0},{t_number,any}},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,1.0},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},2}.
    {fmove,{fr,0},{x,2}}.
    {line,[{location,"spectralnorm.erl",41}]}.
    {bif,element,{f,0},[{tr,{y,3},{t_integer,any}},{y,2}],{x,3}}.
    {fmove,{x,2},{fr,0}}.
    {fconv,{x,3},{fr,1}}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{float,0.0},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},2}.
    {fmove,{fr,0},{x,3}}.
    {move,{y,2},{x,2}}.
    {swap,{x,0},{x,1}}.
    {call,4,{f,15}}. % avloop/4
    {'%',{var_info,{x,0},[{type,{t_float,any}}]}}.
    {move,{x,0},{y,0}}.
  {label,62}.
    {move,{y,2},{x,2}}.
    {move,{y,3},{x,1}}.
    {move,{y,1},{x,0}}.
    {move,{y,0},{y,3}}.
    {trim,3,1}.
    {line,[{location,"spectralnorm.erl",38}]}.
    {call,3,{f,60}}. % '-av/4-lc$^0/1-0-'/3
    {'%',{var_info,{x,0},[{type,{t_list,{t_float,any},nil}}]}}.
    {test_heap,2,1}.
    {put_list,{y,0},{x,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,63}.
    {test,is_nil,{f,64},[{x,0}]}.
    return.
  {label,64}.
    {test_heap,3,1}.
    {put_tuple2,{x,0},{list,[{atom,bad_generator},{x,0}]}}.
    {call_ext_only,1,{extfunc,erlang,error,1}}.


{function, '-av/2-anonymous-0-', 4, 66}.
  {label,65}.
    {line,[{location,"spectralnorm.erl",36}]}.
    {func_info,{atom,spectralnorm},{atom,'-av/2-anonymous-0-'},4}.
  {label,66}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {swap,{x,1},{x,2}}.
    {swap,{x,0},{x,1}}.
    {call_only,4,{f,13}}. % av/4
