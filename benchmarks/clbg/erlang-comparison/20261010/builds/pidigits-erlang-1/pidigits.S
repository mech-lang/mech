{module, pidigits}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 37}.


{function, is_safe, 2, 2}.
  {label,1}.
    {line,[{location,"pidigits.erl",13}]}.
    {func_info,{atom,pidigits},{atom,is_safe},2}.
  {label,2}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{1,'+inf'}}}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {allocate,1,2}.
    {move,{x,1},{y,0}}.
    {move,{integer,4},{x,1}}.
    {call,2,{f,12}}. % extr/2
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {bif,'=:=',
         {f,0},
         [{tr,{y,0},{t_integer,any}},{tr,{x,0},{t_integer,any}}],
         {x,0}}.
    {deallocate,1}.
    return.


{function, next, 1, 4}.
  {label,3}.
    {line,[{location,"pidigits.erl",14}]}.
    {func_info,{atom,pidigits},{atom,next},1}.
  {label,4}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{1,'+inf'}}}}}]}}.
    {move,{integer,3},{x,1}}.
    {call_only,2,{f,12}}. % extr/2


{function, prod, 2, 6}.
  {label,5}.
    {line,[{location,"pidigits.erl",15}]}.
    {func_info,{atom,pidigits},{atom,prod},2}.
  {label,6}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{1,'+inf'}}}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {gc_bif,'*',{f,0},2,[{tr,{x,1},{t_integer,any}},{integer,-10}],{x,1}}.
    {test_heap,5,2}.
    {put_tuple2,{x,1},{list,[{integer,10},{x,1},{integer,0},{integer,1}]}}.
    {swap,{x,0},{x,1}}.
    {call_only,2,{f,10}}. % comp/2


{function, cons, 2, 8}.
  {label,7}.
    {line,[{location,"pidigits.erl",16}]}.
    {func_info,{atom,pidigits},{atom,cons},2}.
  {label,8}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{1,'+inf'}}}}}]}}.
    {'%',{var_info,{x,1},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,{6,'+inf'}},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{3,'+inf'}}}}}]}}.
    {call_only,2,{f,10}}. % comp/2


{function, comp, 2, 10}.
  {label,9}.
    {line,[{location,"pidigits.erl",20}]}.
    {func_info,{atom,pidigits},{atom,comp},2}.
  {label,10}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{1,'+inf'}}}}}]}}.
    {'%',{var_info,{x,1},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{1,'+inf'}}}}}]}}.
    {get_tuple_element,{x,0},0,{x,2}}.
    {get_tuple_element,{x,1},0,{x,3}}.
    {line,[{location,"pidigits.erl",21}]}.
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
            [{tr,{x,2},{t_integer,{1,'+inf'}}},{tr,{x,4},{t_integer,any}}],
            {x,2}}.
    {get_tuple_element,{x,1},3,{x,1}}.
    {get_tuple_element,{x,0},1,{x,4}}.
    {gc_bif,'*',
            {f,0},
            5,
            [{tr,{x,4},{t_integer,any}},{tr,{x,1},{t_integer,{1,'+inf'}}}],
            {x,4}}.
    {gc_bif,'+',
            {f,0},
            5,
            [{tr,{x,2},{t_integer,any}},{tr,{x,4},{t_integer,any}}],
            {x,2}}.
    {get_tuple_element,{x,0},3,{x,0}}.
    {gc_bif,'*',
            {f,0},
            4,
            [{tr,{x,0},{t_integer,{1,'+inf'}}},
             {tr,{x,1},{t_integer,{1,'+inf'}}}],
            {x,0}}.
    {test_heap,5,4}.
    {put_tuple2,{x,0},{list,[{x,3},{x,2},{integer,0},{x,0}]}}.
    return.


{function, extr, 2, 12}.
  {label,11}.
    {line,[{location,"pidigits.erl",22}]}.
    {func_info,{atom,pidigits},{atom,extr},2}.
  {label,12}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{1,'+inf'}}}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,{3,4}}}]}}.
    {get_tuple_element,{x,0},0,{x,2}}.
    {gc_bif,'*',
            {f,0},
            3,
            [{tr,{x,2},{t_integer,{1,'+inf'}}},{tr,{x,1},{t_integer,{3,4}}}],
            {x,1}}.
    {get_tuple_element,{x,0},1,{x,2}}.
    {gc_bif,'+',
            {f,0},
            3,
            [{tr,{x,1},{t_integer,{3,'+inf'}}},{tr,{x,2},{t_integer,any}}],
            {x,1}}.
    {get_tuple_element,{x,0},3,{x,0}}.
    {gc_bif,'div',
            {f,0},
            2,
            [{tr,{x,1},{t_integer,any}},{tr,{x,0},{t_integer,{1,'+inf'}}}],
            {x,0}}.
    return.


{function, lft, 1, 14}.
  {label,13}.
    {line,[{location,"pidigits.erl",24}]}.
    {func_info,{atom,pidigits},{atom,lft},1}.
  {label,14}.
    {'%',{var_info,{x,0},[{type,{t_integer,{1,'+inf'}}}]}}.
    {gc_bif,'*',{f,0},1,[{tr,{x,0},{t_integer,{1,'+inf'}}},{integer,4}],{x,1}}.
    {gc_bif,'+',{f,0},2,[{tr,{x,1},{t_integer,{4,'+inf'}}},{integer,2}],{x,1}}.
    {gc_bif,'*',{f,0},2,[{tr,{x,0},{t_integer,{1,'+inf'}}},{integer,2}],{x,2}}.
    {gc_bif,'+',{f,0},3,[{tr,{x,2},{t_integer,{2,'+inf'}}},{integer,1}],{x,2}}.
    {test_heap,5,3}.
    {put_tuple2,{x,0},{list,[{x,0},{x,1},{integer,0},{x,2}]}}.
    return.


{function, stream, 1, 16}.
  {label,15}.
    {line,[{location,"pidigits.erl",26}]}.
    {func_info,{atom,pidigits},{atom,stream},1}.
  {label,16}.
    {'%',{var_info,{x,0},
                   [{type,{t_union,{t_atom,any},
                                   {t_list,any,any},
                                   {t_number,{1,'+inf'}},
                                   {t_tuple,0,false,#{}},
                                   {t_record,nil,#{}},
                                   other}}]}}.
    {move,{literal,{1,0,0,1}},{x,3}}.
    {move,{integer,1},{x,2}}.
    {move,nil,{x,4}}.
    {move,{integer,0},{x,1}}.
    {call_only,5,{f,18}}. % stream/5


{function, stream, 5, 18}.
  {label,17}.
    {line,[{location,"pidigits.erl",27}]}.
    {func_info,{atom,pidigits},{atom,stream},5}.
  {label,18}.
    {'%',{var_info,{x,0},
                   [{type,{t_union,{t_atom,any},
                                   {t_list,any,any},
                                   {t_number,{1,'+inf'}},
                                   {t_tuple,0,false,#{}},
                                   {t_record,nil,#{}},
                                   other}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,{1,'+inf'}}}]}}.
    {'%',{var_info,{x,3},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{1,'+inf'}}}}}]}}.
    {'%',{var_info,{x,4},[{type,{t_list,{t_integer,any},nil}}]}}.
    {test,is_eq_exact,
          {f,19},
          [{tr,{x,1},{t_integer,{0,'+inf'}}},
           {tr,{x,0},
               {t_union,{t_atom,any},
                        {t_list,any,any},
                        {t_number,{1,'+inf'}},
                        {t_tuple,0,false,#{}},
                        {t_record,nil,#{}},
                        other}}]}.
    {move,{x,4},{x,1}}.
    {call_only,2,{f,25}}. % print/2
  {label,19}.
    {allocate,7,5}.
    {init_yregs,{list,[{y,0},{y,1}]}}.
    {move,{x,4},{y,2}}.
    {move,{x,3},{y,3}}.
    {move,{x,2},{y,4}}.
    {move,{x,1},{y,5}}.
    {move,{x,0},{y,6}}.
    {move,{x,3},{x,0}}.
    {line,[{location,"pidigits.erl",29}]}.
    {call,1,{f,4}}. % next/1
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {move,{x,0},{y,1}}.
    {move,{x,0},{x,1}}.
    {move,{y,3},{x,0}}.
    {line,[{location,"pidigits.erl",30}]}.
    {call,2,{f,2}}. % is_safe/2
    {'%',{var_info,{x,0},[{type,{t_atom,[false,true]}}]}}.
    {test,is_eq_exact,{f,20},[{x,0},{atom,true}]}.
    {line,[{location,"pidigits.erl",32}]}.
    {gc_bif,'+',{f,0},0,[{tr,{y,5},{t_integer,{0,'+inf'}}},{integer,1}],{y,0}}.
    {move,{y,1},{x,1}}.
    {move,{y,3},{x,0}}.
    {move,{y,0},{y,3}}.
    {trim,1,6}.
    {call,2,{f,6}}. % prod/2
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{1,'+inf'}}}}}]}}.
    {move,{y,0},{x,1}}.
    {move,{y,4},{x,3}}.
    {move,{x,0},{y,4}}.
    {move,{y,1},{x,2}}.
    {trim,2,4}.
    {move,{x,3},{x,0}}.
    {call,3,{f,22}}. % update/3
    {'%',{var_info,{x,0},[{type,{t_cons,{t_integer,any},nil}}]}}.
    {move,{y,0},{x,1}}.
    {move,{y,1},{x,2}}.
    {move,{y,2},{x,3}}.
    {move,{x,0},{x,4}}.
    {move,{y,3},{x,0}}.
    {call_last,5,{f,18},4}. % stream/5
  {label,20}.
    {line,[{location,"pidigits.erl",34}]}.
    {gc_bif,'+',{f,0},0,[{tr,{y,4},{t_integer,{1,'+inf'}}},{integer,1}],{y,1}}.
    {move,{y,4},{x,0}}.
    {move,{y,1},{y,4}}.
    {trim,2,5}.
    {call,1,{f,14}}. % lft/1
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,{6,'+inf'}},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{3,'+inf'}}}}}]}}.
    {move,{x,0},{x,1}}.
    {move,{y,1},{x,0}}.
    {move,{y,0},{y,1}}.
    {trim,1,4}.
    {call,2,{f,8}}. % cons/2
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,4,true,
                                   #{1 => {t_integer,{1,'+inf'}},
                                     2 => {t_integer,any},
                                     3 => {t_integer,{0,0}},
                                     4 => {t_integer,{1,'+inf'}}}}}]}}.
    {move,{y,1},{x,2}}.
    {move,{x,0},{x,3}}.
    {move,{y,0},{x,4}}.
    {move,{y,2},{x,1}}.
    {move,{y,3},{x,0}}.
    {call_last,5,{f,18},4}. % stream/5


{function, update, 3, 22}.
  {label,21}.
    {line,[{location,"pidigits.erl",38}]}.
    {func_info,{atom,pidigits},{atom,update},3}.
  {label,22}.
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_list,{t_integer,any},nil}}]}}.
    {gc_bif,'rem',
            {f,0},
            3,
            [{tr,{x,0},{t_integer,{0,'+inf'}}},{integer,10}],
            {x,3}}.
    {test,is_eq_exact,{f,23},[{tr,{x,3},{t_integer,{0,9}}},{integer,0}]}.
    {test,is_ge,{f,23},[{tr,{x,0},{t_integer,{0,'+inf'}}},{integer,1}]}.
    {allocate,1,3}.
    {move,{x,1},{y,0}}.
    {move,{x,2},{x,1}}.
    {line,[{location,"pidigits.erl",39}]}.
    {call,2,{f,25}}. % print/2
    {test_heap,2,0}.
    {put_list,{y,0},nil,{x,0}}.
    {deallocate,1}.
    return.
  {label,23}.
    {test_heap,2,3}.
    {put_list,{x,1},{x,2},{x,0}}.
    return.


{function, print, 2, 25}.
  {label,24}.
    {line,[{location,"pidigits.erl",45}]}.
    {func_info,{atom,pidigits},{atom,print},2}.
  {label,25}.
    {'%',{var_info,{x,0},[{type,{t_integer,{1,'+inf'}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_list,{t_integer,any},nil}}]}}.
    {allocate,1,2}.
    {move,{x,0},{y,0}}.
    {move,{x,1},{x,0}}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {move,{x,0},{x,1}}.
    {move,{y,0},{x,0}}.
    {call_last,2,{f,27},1}. % do_print/2


{function, do_print, 2, 27}.
  {label,26}.
    {line,[{location,"pidigits.erl",48}]}.
    {func_info,{atom,pidigits},{atom,do_print},2}.
  {label,27}.
    {'%',{var_info,{x,0},[{type,{t_integer,{1,'+inf'}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_list,{t_integer,any},nil}}]}}.
    {test,is_nonempty_list,{f,28},[{x,1}]}.
    {allocate_heap,2,2,2}.
    {move,{x,0},{y,1}}.
    {get_list,{x,1},{x,0},{y,0}}.
    {put_list,{x,0},nil,{x,1}}.
    {move,{literal,"~p"},{x,0}}.
    {line,[{location,"pidigits.erl",50}]}.
    {call_ext,2,{extfunc,io,fwrite,2}}.
    {move,{y,0},{x,1}}.
    {move,{y,1},{x,0}}.
    {call_last,2,{f,27},2}. % do_print/2
  {label,28}.
    {line,[{location,"pidigits.erl",48}]}.
    {gc_bif,'rem',
            {f,0},
            1,
            [{tr,{x,0},{t_integer,{1,'+inf'}}},{integer,10}],
            {x,1}}.
    {test,is_eq_exact,{f,29},[{tr,{x,1},{t_integer,{0,9}}},{integer,0}]}.
    {test_heap,2,1}.
    {put_list,{x,0},nil,{x,1}}.
    {move,{literal,"\t:~p~n"},{x,0}}.
    {call_ext_only,2,{extfunc,io,fwrite,2}}.
  {label,29}.
    {line,[{location,"pidigits.erl",49}]}.
    {gc_bif,'-',{f,0},2,[{integer,10},{tr,{x,1},{t_integer,{1,9}}}],{x,1}}.
    {test_heap,6,2}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{integer,9},{x,0},{x,0}}.
    {put_list,{x,1},{x,0},{x,1}}.
    {move,{literal,"~*.1c:~p~n"},{x,0}}.
    {call_ext_only,2,{extfunc,io,fwrite,2}}.


{function, main, 1, 31}.
  {label,30}.
    {line,[{location,"pidigits.erl",53}]}.
    {func_info,{atom,pidigits},{atom,main},1}.
  {label,31}.
    {test,is_nonempty_list,{f,32},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,32},[{x,2}]}.
    {allocate,0,2}.
    {move,{x,1},{x,0}}.
    {line,[{location,"pidigits.erl",54}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {line,[{location,"pidigits.erl",55}]}.
    {call,1,{f,31}}. % main/1
    {move,{integer,0},{x,0}}.
    {line,[{location,"pidigits.erl",56}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.
  {label,32}.
    {test,is_lt,{f,30},[{integer,1},{x,0}]}.
    {call_only,1,{f,16}}. % stream/1


{function, module_info, 0, 34}.
  {label,33}.
    {line,[]}.
    {func_info,{atom,pidigits},{atom,module_info},0}.
  {label,34}.
    {move,{atom,pidigits},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 36}.
  {label,35}.
    {line,[]}.
    {func_info,{atom,pidigits},{atom,module_info},1}.
  {label,36}.
    {move,{x,0},{x,1}}.
    {move,{atom,pidigits},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.
