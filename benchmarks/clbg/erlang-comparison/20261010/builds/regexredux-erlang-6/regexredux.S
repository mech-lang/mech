{module, regexredux}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 67}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"regexredux.erl",26}]}.
    {func_info,{atom,regexredux},{atom,main},1}.
  {label,2}.
    {call_only,0,{f,4}}. % main/0


{function, main, 0, 4}.
  {label,3}.
    {line,[{location,"regexredux.erl",28}]}.
    {func_info,{atom,regexredux},{atom,main},0}.
  {label,4}.
    {allocate,0,0}.
    {call,0,{f,6}}. % do/0
    {'%',{var_info,{x,0},[{type,{t_atom,[ok]}}]}}.
    {call_ext_last,0,{extfunc,erlang,halt,0},0}.


{function, do, 0, 6}.
  {label,5}.
    {line,[{location,"regexredux.erl",30}]}.
    {func_info,{atom,regexredux},{atom,do},0}.
  {label,6}.
    {allocate_heap,1,{alloc,[{words,1},{floats,0},{funs,1}]},0}.
    {init_yregs,{list,[{y,0}]}}.
    {bif,self,{f,0},[],{x,0}}.
    {make_fun3,{f,66},0,0,{x,0},{list,[{x,0}]}}.
    {line,[{location,"regexredux.erl",32}]}.
    {call_ext,1,{extfunc,erlang,spawn_link,1}}.
    {move,{x,0},{y,0}}.
    {line,[{location,"regexredux.erl",33}]}.
    {call,0,{f,38}}. % read/0
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_tuple,2,true,
                     #{1 => {t_integer,any},
                       2 => {t_cons,{t_bitstring,8,false},nil}}}}]}}.
    {test_heap,3,1}.
    {put_tuple2,{x,1},{list,[{atom,data},{x,0}]}}.
    {move,{y,0},{x,0}}.
    {init_yregs,{list,[{y,0}]}}.
    send.
  {label,7}.
    {loop_rec,{f,9},{x,0}}.
    {test,is_eq_exact,{f,8},[{x,0},{atom,finish}]}.
    remove_message.
    {move,{atom,ok},{x,0}}.
    {deallocate,1}.
    return.
  {label,8}.
    {loop_rec_end,{f,7}}.
  {label,9}.
    {wait,{f,7}}.


{function, work, 1, 11}.
  {label,10}.
    {line,[{location,"regexredux.erl",36}]}.
    {func_info,{atom,regexredux},{atom,work},1}.
  {label,11}.
    {'%',{var_info,{x,0},[{type,pid}]}}.
    {allocate,7,1}.
    {init_yregs,{list,[{y,0},{y,1},{y,2},{y,3},{y,4}]}}.
    {move,{x,0},{y,6}}.
    {bif,self,{f,0},[],{y,5}}.
    {line,[{location,"regexredux.erl",39}]}.
    {call,0,{f,34}}. % patterns/0
    {'%',{var_info,{x,0},
                   [{type,{t_cons,{t_cons,{t_integer,{91,124}},nil},nil}}]}}.
    {call,1,{f,63}}. % '-work/1-lc$^0/1-0-'/1
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_cons,{t_integer,{91,124}},nil}}},
                     nil}}]}}.
    {move,{x,0},{y,4}}.
  {label,12}.
    {loop_rec,{f,14},{x,0}}.
    {test,is_tagged_tuple,{f,13},[{x,0},2,{atom,data}]}.
    remove_message.
    {get_tuple_element,{x,0},1,{x,0}}.
    {test,is_tuple,{f,15},[{x,0}]}.
    {test,test_arity,{f,15},[{x,0},2]}.
    {get_tuple_element,{x,0},0,{y,3}}.
    {get_tuple_element,{x,0},1,{x,1}}.
    {test,is_nonempty_list,{f,15},[{x,1}]}.
    {get_list,{x,1},{y,2},{x,1}}.
    {test,is_nonempty_list,{f,15},[{x,1}]}.
    {get_list,{x,1},{y,1},{x,1}}.
    {test,is_nonempty_list,{f,15},[{x,1}]}.
    {test_heap,6,2}.
    {get_hd,{x,1},{y,0}}.
    {put_list,{y,2},nil,{x,0}}.
    {put_list,{y,1},{x,0},{x,0}}.
    {put_list,{y,0},{x,0},{x,0}}.
    {line,[{location,"regexredux.erl",43}]}.
    {call,1,{f,60}}. % '-work/1-lc$^1/1-1-'/1
    {'%',{var_info,{x,0},[{type,{t_cons,{t_integer,any},nil}}]}}.
    {line,[{location,"regexredux.erl",44}]}.
    {call_ext,1,{extfunc,lists,sum,1}}.
    {move,{y,1},{x,2}}.
    {move,{y,2},{x,1}}.
    {move,{y,4},{x,4}}.
    {move,{y,5},{x,3}}.
    {move,{x,0},{y,5}}.
    {init_yregs,{list,[{y,4}]}}.
    {move,{x,4},{x,0}}.
    {line,[{location,"regexredux.erl",47}]}.
    {call,4,{f,56}}. % '-work/1-lc$^2/1-2-'/4
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => pid,
                           2 => {t_fun,1,{'-printer/1-fun-0-',2},any}}},
                     nil}}]}}.
    {move,{x,0},{y,4}}.
    {line,[{location,"regexredux.erl",48}]}.
    {call,0,{f,36}}. % final_patterns/0
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_cons,{t_integer,{42,124}},nil},
                           2 => {t_cons,{t_integer,{45,124}},nil}}},
                     nil}}]}}.
    {test_heap,4,1}.
    {put_list,{y,2},nil,{x,1}}.
    {put_list,{y,1},{x,1},{x,1}}.
    {swap,{x,0},{x,1}}.
    {init_yregs,{list,[{y,1},{y,2}]}}.
    {call,2,{f,17}}. % apply_final_patterns/2
    {line,[{location,"regexredux.erl",49}]}.
    {gc_bif,size,{f,0},1,[{y,0}],{x,1}}.
    {gc_bif,size,{f,0},2,[{x,0}],{x,0}}.
    {gc_bif,'+',
            {f,0},
            2,
            [{tr,{x,1},{t_integer,any}},{tr,{x,0},{t_integer,any}}],
            {y,2}}.
    {move,{y,4},{x,0}}.
    {move,{y,2},{y,4}}.
    {trim,3,4}.
    {line,[{location,"regexredux.erl",50}]}.
    {call,1,{f,28}}. % results/1
    {'%',{var_info,{x,0},[{type,{t_atom,[ok]}}]}}.
    {test_heap,6,0}.
    {put_list,{y,1},nil,{x,0}}.
    {put_list,{y,2},{x,0},{x,0}}.
    {put_list,{y,0},{x,0},{x,1}}.
    {trim,3,1}.
    {move,{literal,"~n~b~n~b~n~b~n"},{x,0}}.
    {line,[{location,"regexredux.erl",51}]}.
    {call_ext,2,{extfunc,io,format,2}}.
    {move,{atom,finish},{x,1}}.
    {move,{y,0},{x,0}}.
    {line,[{location,"regexredux.erl",53}]}.
    send.
    {deallocate,1}.
    return.
  {label,13}.
    {loop_rec_end,{f,12}}.
  {label,14}.
    {wait,{f,12}}.
  {label,15}.
    {line,[{location,"regexredux.erl",40}]}.
    {badmatch,{x,0}}.


{function, apply_final_patterns, 2, 17}.
  {label,16}.
    {line,[{location,"regexredux.erl",55}]}.
    {func_info,{atom,regexredux},{atom,apply_final_patterns},2}.
  {label,17}.
    {'%',
        {var_info,
            {x,1},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => {t_cons,{t_integer,{42,124}},nil},
                           2 => {t_cons,{t_integer,{45,124}},nil}}},
                     nil}}]}}.
    {test,is_nonempty_list,{f,18},[{x,1}]}.
    {allocate,1,2}.
    {get_list,{x,1},{x,1},{y,0}}.
    {get_tuple_element,{x,1},0,{x,2}}.
    {get_tuple_element,{x,1},1,{x,1}}.
    {move,{literal,[{return,binary},caseless,global,never_utf]},{x,3}}.
    {swap,{x,1},{x,2}}.
    {line,[{location,"regexredux.erl",56}]}.
    {call_ext,4,{extfunc,re,replace,4}}.
    {move,{y,0},{x,1}}.
    {call_last,2,{f,17},1}. % apply_final_patterns/2
  {label,18}.
    return.


{function, matcher, 4, 20}.
  {label,19}.
    {line,[{location,"regexredux.erl",60}]}.
    {func_info,{atom,regexredux},{atom,matcher},4}.
  {label,20}.
    {'%',{var_info,{x,0},[{type,pid}]}}.
    {test_heap,{alloc,[{words,4},{floats,0},{funs,1}]},4}.
    {make_fun3,{f,54},0,0,{x,0},{list,[{x,0},{x,1},{x,2},{x,3}]}}.
    return.


{function, printer, 1, 22}.
  {label,21}.
    {line,[{location,"regexredux.erl",66}]}.
    {func_info,{atom,regexredux},{atom,printer},1}.
  {label,22}.
    {'%',{var_info,{x,0},[{type,{t_cons,{t_integer,{91,124}},nil}}]}}.
    {test_heap,{alloc,[{words,1},{floats,0},{funs,1}]},1}.
    {make_fun3,{f,52},0,0,{x,0},{list,[{x,0}]}}.
    return.


{function, countMatches, 2, 24}.
  {label,23}.
    {line,[{location,"regexredux.erl",69}]}.
    {func_info,{atom,regexredux},{atom,countMatches},2}.
  {label,24}.
    {allocate,0,2}.
    {move,{literal,[global]},{x,2}}.
    {line,[{location,"regexredux.erl",70}]}.
    {call_ext,3,{extfunc,re,run,3}}.
    {test,is_tuple,{f,25},[{x,0}]}.
    {test,is_tagged_tuple,
          {f,26},
          [{tr,{x,0},{t_tuple,0,false,#{}}},2,{atom,match}]}.
    {get_tuple_element,{x,0},1,{x,0}}.
    {line,[{location,"regexredux.erl",71}]}.
    {gc_bif,length,{f,0},1,[{x,0}],{x,0}}.
    {deallocate,0}.
    return.
  {label,25}.
    {test,is_eq_exact,{f,26},[{x,0},{atom,nomatch}]}.
    {move,{integer,0},{x,0}}.
    {deallocate,0}.
    return.
  {label,26}.
    {line,[{location,"regexredux.erl",70}]}.
    {case_end,{x,0}}.


{function, results, 1, 28}.
  {label,27}.
    {line,[{location,"regexredux.erl",75}]}.
    {func_info,{atom,regexredux},{atom,results},1}.
  {label,28}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => pid,
                           2 => {t_fun,1,{'-printer/1-fun-0-',2},any}}},
                     nil}}]}}.
    {test,is_nonempty_list,{f,32},[{x,0}]}.
    {allocate,3,1}.
    {get_list,{x,0},{y,2},{y,1}}.
    {get_tuple_element,{y,2},0,{y,0}}.
  {label,29}.
    {loop_rec,{f,31},{x,0}}.
    {test,is_tuple,{f,30},[{x,0}]}.
    {test,test_arity,{f,30},[{x,0},2]}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {test,is_eq_exact,{f,30},[{x,1},{tr,{y,0},pid}]}.
    remove_message.
    {get_tuple_element,{x,0},1,{x,0}}.
    {get_tuple_element,{y,2},1,{x,1}}.
    {move,{y,1},{y,2}}.
    {trim,2,1}.
    {line,[{location,"regexredux.erl",76}]}.
    {call_fun2,{f,52},1,{tr,{x,1},{t_fun,1,{'-printer/1-fun-0-',2},any}}}.
    {move,{y,0},{x,0}}.
    {call_last,1,{f,28},1}. % results/1
  {label,30}.
    {loop_rec_end,{f,29}}.
  {label,31}.
    {wait,{f,29}}.
  {label,32}.
    {move,{atom,ok},{x,0}}.
    return.


{function, patterns, 0, 34}.
  {label,33}.
    {line,[{location,"regexredux.erl",79}]}.
    {func_info,{atom,regexredux},{atom,patterns},0}.
  {label,34}.
    {move,{literal,["agggtaaa|tttaccct","[cgt]gggtaaa|tttaccc[acg]",
                    "a[act]ggtaaa|tttacc[agt]t","ag[act]gtaaa|tttac[agt]ct",
                    "agg[act]taaa|ttta[agt]cct","aggg[acg]aaa|ttt[cgt]ccct",
                    "agggt[cgt]aa|tt[acg]accct","agggta[cgt]a|t[acg]taccct",
                    "agggtaa[cgt]|[acg]ttaccct"]},
          {x,0}}.
    return.


{function, final_patterns, 0, 36}.
  {label,35}.
    {line,[{location,"regexredux.erl",89}]}.
    {func_info,{atom,regexredux},{atom,final_patterns},0}.
  {label,36}.
    {move,{literal,[{"tHa[Nt]","<4>"},
                    {"aND|caN|Ha[DS]|WaS","<3>"},
                    {"a[NSt]|BY","<2>"},
                    {"<[^>]*>","|"},
                    {"\\|[^|][^|]*\\|","-"}]},
          {x,0}}.
    return.


{function, read, 0, 38}.
  {label,37}.
    {line,[{location,"regexredux.erl",98}]}.
    {func_info,{atom,regexredux},{atom,read},0}.
  {label,38}.
    {allocate,0,0}.
    {move,{literal,[in,binary,{line,256}]},{x,1}}.
    {move,{literal,{fd,0,1}},{x,0}}.
    {line,[{location,"regexredux.erl",99}]}.
    {call_ext,2,{extfunc,erlang,open_port,2}}.
    {move,nil,{x,2}}.
    {move,nil,{x,3}}.
    {move,{integer,0},{x,1}}.
    {call_last,4,{f,40},0}. % read/4


{function, read, 4, 40}.
  {label,39}.
    {line,[{location,"regexredux.erl",102}]}.
    {func_info,{atom,regexredux},{atom,read},4}.
  {label,40}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',
        {var_info,
            {x,2},
            [{type,
                 {t_list,
                     {t_union,none,none,none,
                         {t_tuple,0,false,#{}},
                         none,
                         {t_bitstring,1,false}},
                     nil}}]}}.
    {'%',{var_info,{x,3},[{type,{t_list,{t_bitstring,8,false},nil}}]}}.
    {allocate,4,4}.
    {move,{x,3},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,1},{y,2}}.
    {move,{x,0},{y,3}}.
  {label,41}.
    {loop_rec,{f,46},{x,0}}.
    {test,is_tuple,{f,45},[{x,0}]}.
    {select_tuple_arity,{tr,{x,0},{t_tuple,0,false,#{}}},
                        {f,45},
                        {list,[2,{f,43},3,{f,42}]}}.
  {label,42}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{x,2}}.
    {get_tuple_element,{x,0},2,{x,3}}.
    {test,is_eq_exact,{f,45},[{x,1},{atom,'EXIT'}]}.
    {test,is_eq_exact,{f,45},[{x,3},{atom,normal}]}.
    {test,is_eq_exact,{f,45},[{x,2},{y,3}]}.
    remove_message.
    {move,nil,{x,1}}.
    {move,{y,1},{x,0}}.
    {move,{y,0},{y,3}}.
    {trim,2,2}.
    {line,[{location,"regexredux.erl",110}]}.
    {call_ext,2,{extfunc,lists,reverse,2}}.
    {call_ext,1,{extfunc,erlang,iolist_to_binary,1}}.
    {test_heap,5,1}.
    {put_list,{x,0},{y,1},{x,0}}.
    {put_tuple2,{x,0},{list,[{y,0},{x,0}]}}.
    {deallocate,2}.
    return.
  {label,43}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{x,2}}.
    {test,is_tagged_tuple,{f,45},[{x,2},2,{atom,data}]}.
    {get_tuple_element,{x,2},1,{x,2}}.
    {test,is_tagged_tuple,{f,45},[{x,2},2,{atom,eol}]}.
    {get_tuple_element,{x,2},1,{x,2}}.
    {test,bs_start_match3,{f,44},3,[{x,2}],{x,3}}.
    {bs_match,{f,44},
              {x,3},
              {commands,[{ensure_at_least,8,8},{'=:=',nil,8,62}]}}.
    {test,is_eq_exact,{f,45},[{x,1},{y,3}]}.
    {move,{x,2},{x,0}}.
    remove_message.
    {line,[{location,"regexredux.erl",105}]}.
    {gc_bif,size,{f,0},1,[{tr,{x,0},{t_bs_matchable,1}}],{x,0}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,2},{t_integer,any}},{tr,{x,0},{t_integer,any}}],
            {x,0}}.
    {gc_bif,'+',{f,0},1,[{tr,{x,0},{t_integer,any}},{integer,1}],{y,2}}.
    {move,nil,{x,1}}.
    {move,{y,1},{x,0}}.
    {move,{y,0},{y,1}}.
    {trim,1,3}.
    {line,[{location,"regexredux.erl",106}]}.
    {call_ext,2,{extfunc,lists,reverse,2}}.
    {call_ext,1,{extfunc,erlang,iolist_to_binary,1}}.
    {test_heap,2,1}.
    {put_list,{x,0},{y,0},{x,3}}.
    {move,{y,1},{x,1}}.
    {move,nil,{x,2}}.
    {move,{y,2},{x,0}}.
    {call_last,4,{f,40},3}. % read/4
  {label,44}.
    {test,is_eq_exact,{f,45},[{x,1},{y,3}]}.
    {move,{x,2},{x,0}}.
    remove_message.
    {line,[{location,"regexredux.erl",108}]}.
    {gc_bif,size,{f,0},1,[{x,0}],{x,1}}.
    {gc_bif,'+',
            {f,0},
            2,
            [{tr,{y,2},{t_integer,any}},{tr,{x,1},{t_integer,any}}],
            {x,1}}.
    {gc_bif,'+',{f,0},2,[{tr,{x,1},{t_integer,any}},{integer,1}],{x,1}}.
    {test_heap,2,2}.
    {put_list,{x,0},{y,1},{x,2}}.
    {move,{y,0},{x,3}}.
    {move,{y,3},{x,0}}.
    {call_last,4,{f,40},4}. % read/4
  {label,45}.
    {test_heap,2,1}.
    remove_message.
    {put_list,{x,0},nil,{x,1}}.
    {trim,4,0}.
    {move,{literal,">>>>>>> Wrong! ~p~n"},{x,0}}.
    {line,[{location,"regexredux.erl",112}]}.
    {call_ext,2,{extfunc,io,format,2}}.
    {move,{atom,bad_data},{x,0}}.
    {line,[{location,"regexredux.erl",113}]}.
    {call_ext_last,1,{extfunc,erlang,exit,1},0}.
  {label,46}.
    {wait,{f,41}}.


{function, module_info, 0, 48}.
  {label,47}.
    {line,[]}.
    {func_info,{atom,regexredux},{atom,module_info},0}.
  {label,48}.
    {move,{atom,regexredux},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 50}.
  {label,49}.
    {line,[]}.
    {func_info,{atom,regexredux},{atom,module_info},1}.
  {label,50}.
    {move,{x,0},{x,1}}.
    {move,{atom,regexredux},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-printer/1-fun-0-', 2, 52}.
  {label,51}.
    {line,[{location,"regexredux.erl",67}]}.
    {func_info,{atom,regexredux},{atom,'-printer/1-fun-0-'},2}.
  {label,52}.
    {'%',{var_info,{x,1},[{type,{t_cons,{t_integer,{91,124}},nil}}]}}.
    {test_heap,4,2}.
    {put_list,{x,0},nil,{x,0}}.
    {put_list,{x,1},{x,0},{x,1}}.
    {move,{literal,"~s ~b~n"},{x,0}}.
    {call_ext_only,2,{extfunc,io,format,2}}.


{function, '-matcher/4-fun-0-', 4, 54}.
  {label,53}.
    {line,[{location,"regexredux.erl",61}]}.
    {func_info,{atom,regexredux},{atom,'-matcher/4-fun-0-'},4}.
  {label,54}.
    {'%',{var_info,{x,0},[{type,pid}]}}.
    {allocate,4,4}.
    {move,{x,3},{y,1}}.
    {move,{x,2},{y,2}}.
    {move,{x,0},{y,3}}.
    {bif,self,{f,0},[],{y,0}}.
    {move,{x,1},{x,0}}.
    {move,{x,3},{x,1}}.
    {line,[{location,"regexredux.erl",63}]}.
    {call,2,{f,24}}. % countMatches/2
    {'%',{var_info,{x,0},[{type,{t_integer,{0,288230376151711743}}}]}}.
    {swap,{y,2},{x,0}}.
    {move,{y,1},{x,1}}.
    {move,{y,0},{y,1}}.
    {trim,1,3}.
    {call,2,{f,24}}. % countMatches/2
    {'%',{var_info,{x,0},[{type,{t_integer,{0,288230376151711743}}}]}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{y,1},{t_integer,{0,288230376151711743}}},
             {tr,{x,0},{t_integer,{0,288230376151711743}}}],
            {x,0}}.
    {test_heap,3,1}.
    {put_tuple2,{x,1},{list,[{y,0},{x,0}]}}.
    {move,{y,2},{x,0}}.
    {line,[{location,"regexredux.erl",62}]}.
    send.
    {deallocate,3}.
    return.


{function, '-work/1-lc$^2/1-2-', 4, 56}.
  {label,55}.
    {line,[{location,"regexredux.erl",47}]}.
    {func_info,{atom,regexredux},{atom,'-work/1-lc$^2/1-2-'},4}.
  {label,56}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => {t_cons,{t_integer,{91,124}},nil}}},
                     nil}}]}}.
    {'%',{var_info,{x,3},[{type,pid}]}}.
    {test,is_nonempty_list,{f,58},[{x,0}]}.
    {get_list,{x,0},{x,4},{x,0}}.
    {get_tuple_element,{x,4},0,{x,5}}.
    {get_tuple_element,{x,4},1,{x,4}}.
    {test,is_tagged_tuple,{f,57},[{x,4},2,{atom,ok}]}.
    {allocate,5,6}.
    {move,{x,3},{y,0}}.
    {move,{x,5},{y,1}}.
    {move,{x,0},{y,2}}.
    {move,{x,2},{y,3}}.
    {move,{x,1},{y,4}}.
    {get_tuple_element,{x,4},1,{x,3}}.
    {swap,{x,2},{x,1}}.
    {move,{y,0},{x,0}}.
    {line,[{location,"regexredux.erl",45}]}.
    {call,4,{f,20}}. % matcher/4
    {'%',{var_info,{x,0},[{type,{t_fun,0,{'-matcher/4-fun-0-',4},any}}]}}.
    {call_ext,1,{extfunc,erlang,spawn_link,1}}.
    {swap,{y,1},{x,0}}.
    {line,[{location,"regexredux.erl",46}]}.
    {call,1,{f,22}}. % printer/1
    {'%',{var_info,{x,0},[{type,{t_fun,1,{'-printer/1-fun-0-',2},any}}]}}.
    {move,{y,2},{x,4}}.
    {move,{y,3},{x,2}}.
    {move,{y,4},{x,1}}.
    {move,{x,0},{y,4}}.
    {move,{y,0},{x,3}}.
    {move,{y,1},{y,3}}.
    {trim,3,2}.
    {move,{x,4},{x,0}}.
    {line,[{location,"regexredux.erl",47}]}.
    {call,4,{f,56}}. % '-work/1-lc$^2/1-2-'/4
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => pid,
                           2 => {t_fun,1,{'-printer/1-fun-0-',2},any}}},
                     nil}}]}}.
    {test_heap,5,1}.
    {put_tuple2,{x,1},{list,[{y,0},{y,1}]}}.
    {put_list,{x,1},{x,0},{x,0}}.
    {deallocate,2}.
    return.
  {label,57}.
    {call_only,4,{f,56}}. % '-work/1-lc$^2/1-2-'/4
  {label,58}.
    return.


{function, '-work/1-lc$^1/1-1-', 1, 60}.
  {label,59}.
    {line,[{location,"regexredux.erl",43}]}.
    {func_info,{atom,regexredux},{atom,'-work/1-lc$^1/1-1-'},1}.
  {label,60}.
    {'%',{var_info,{x,0},[{type,{t_list,any,nil}}]}}.
    {test,is_nonempty_list,{f,61},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,0}}.
    {gc_bif,size,{f,0},2,[{x,1}],{x,1}}.
    {allocate,1,2}.
    {move,{x,1},{y,0}}.
    {call,1,{f,60}}. % '-work/1-lc$^1/1-1-'/1
    {'%',{var_info,{x,0},[{type,{t_list,{t_integer,any},nil}}]}}.
    {test_heap,2,1}.
    {put_list,{y,0},{x,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,61}.
    return.


{function, '-work/1-lc$^0/1-0-', 1, 63}.
  {label,62}.
    {line,[{location,"regexredux.erl",39}]}.
    {func_info,{atom,regexredux},{atom,'-work/1-lc$^0/1-0-'},1}.
  {label,63}.
    {'%',{var_info,{x,0},
                   [{type,{t_list,{t_cons,{t_integer,{91,124}},nil},nil}}]}}.
    {test,is_nonempty_list,{f,64},[{x,0}]}.
    {allocate,2,1}.
    {get_list,{x,0},{y,1},{y,0}}.
    {move,{literal,[caseless]},{x,1}}.
    {move,{y,1},{x,0}}.
    {line,[{location,"regexredux.erl",38}]}.
    {call_ext,2,{extfunc,re,compile,2}}.
    {swap,{y,0},{x,0}}.
    {line,[{location,"regexredux.erl",39}]}.
    {call,1,{f,63}}. % '-work/1-lc$^0/1-0-'/1
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => {t_cons,{t_integer,{91,124}},nil}}},
                     nil}}]}}.
    {test_heap,5,1}.
    {put_tuple2,{x,1},{list,[{y,1},{y,0}]}}.
    {put_list,{x,1},{x,0},{x,0}}.
    {deallocate,2}.
    return.
  {label,64}.
    return.


{function, '-do/0-fun-0-', 1, 66}.
  {label,65}.
    {line,[{location,"regexredux.erl",32}]}.
    {func_info,{atom,regexredux},{atom,'-do/0-fun-0-'},1}.
  {label,66}.
    {'%',{var_info,{x,0},[{type,pid}]}}.
    {call_only,1,{f,11}}. % work/1
