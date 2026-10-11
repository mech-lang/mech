{module, knucleotide}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 62}.


{function, to_upper, 2, 2}.
  {label,1}.
    {line,[{location,"knucleotide.erl",9}]}.
    {func_info,{atom,knucleotide},{atom,to_upper},2}.
  {label,2}.
    {'%',{var_info,{x,0},[accepts_match_context]}}.
    {'%',{var_info,{x,1},[{type,{t_list,{t_integer,any},nil}}]}}.
    {test,bs_start_match3,{f,1},2,[{x,0}],{x,2}}.
    {bs_get_position,{x,2},{x,0},3}.
    {bs_match,{f,5},
              {x,2},
              {commands,[{ensure_at_least,8,1},
                         {integer,3,{literal,[]},8,1,{x,3}}]}}.
    {bs_match,{f,3},{x,2},{commands,[{ensure_at_least,0,8}]}}.
    {test,is_ge,{f,3},[{tr,{x,3},{t_integer,{0,255}}},{integer,97}]}.
    {test,is_ge,{f,4},[{integer,122},{tr,{x,3},{t_integer,{97,255}}}]}.
    {line,[{location,"knucleotide.erl",10}]}.
    {gc_bif,'-',{f,0},4,[{tr,{x,3},{t_integer,{97,122}}},{integer,32}],{x,0}}.
    {test_heap,2,3}.
    {put_list,{x,0},{x,1},{x,1}}.
    {move,{x,2},{x,0}}.
    {call_only,2,{f,2}}. % to_upper/2
  {label,3}.
    {test,is_eq_exact,{f,4},[{tr,{x,3},{t_integer,{0,255}}},{integer,10}]}.
    {bs_match,{f,6},{x,2},{commands,[{ensure_at_least,0,8}]}}.
    {move,{x,2},{x,0}}.
    {call_only,2,{f,2}}. % to_upper/2
  {label,4}.
    {bs_match,{f,6},{x,2},{commands,[{ensure_at_least,0,8}]}}.
    {test_heap,2,4}.
    {put_list,{x,3},{x,1},{x,1}}.
    {move,{x,2},{x,0}}.
    {call_only,2,{f,2}}. % to_upper/2
  {label,5}.
    {bs_set_position,{x,2},{x,0}}.
    {bs_match,{f,6},{x,2},{commands,[{ensure_exactly,0}]}}.
    {allocate,0,2}.
    {move,{x,1},{x,0}}.
    {line,[{location,"knucleotide.erl",13}]}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {call_ext_last,1,{extfunc,erlang,list_to_binary,1},0}.
  {label,6}.
    {bs_set_position,{x,2},{x,0}}.
    {bs_get_tail,{x,2},{x,0},3}.
    {jump,{f,1}}.


{function, seek_three, 0, 8}.
  {label,7}.
    {line,[{location,"knucleotide.erl",16}]}.
    {func_info,{atom,knucleotide},{atom,seek_three},0}.
  {label,8}.
    {allocate,0,0}.
    {move,{atom,''},{x,0}}.
    {line,[{location,"knucleotide.erl",17}]}.
    {call_ext,1,{extfunc,io,get_line,1}}.
    {test,bs_start_match3,{f,9},1,[{x,0}],{x,1}}.
    {bs_match,{f,10},
              {x,1},
              {commands,[{ensure_at_least,24,8},{'=:=',nil,24,4084808}]}}.
    {move,{atom,done},{x,0}}.
    {deallocate,0}.
    return.
  {label,9}.
    {test,is_eq_exact,{f,10},[{x,0},{atom,eof}]}.
    {line,[{location,"knucleotide.erl",19}]}.
    {call_ext_last,1,{extfunc,erlang,error,1},0}.
  {label,10}.
    {call_last,0,{f,8},0}. % seek_three/0


{function, get_seq_three, 1, 12}.
  {label,11}.
    {line,[{location,"knucleotide.erl",24}]}.
    {func_info,{atom,knucleotide},{atom,get_seq_three},1}.
  {label,12}.
    {'%',{var_info,{x,0},[{type,{t_list,{t_bitstring,8,false},nil}}]}}.
    {allocate,1,1}.
    {move,{x,0},{y,0}}.
    {move,{atom,''},{x,0}}.
    {line,[{location,"knucleotide.erl",25}]}.
    {call_ext,1,{extfunc,io,get_line,1}}.
    {test,is_eq_exact,{f,13},[{x,0},{atom,eof}]}.
    {move,{y,0},{x,0}}.
    {trim,1,0}.
    {line,[{location,"knucleotide.erl",26}]}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {call_ext_last,1,{extfunc,erlang,iolist_to_binary,1},0}.
  {label,13}.
    {move,nil,{x,1}}.
    {line,[{location,"knucleotide.erl",27}]}.
    {call,2,{f,2}}. % to_upper/2
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {test_heap,2,1}.
    {put_list,{x,0},{y,0},{x,0}}.
    {call_last,1,{f,12},1}. % get_seq_three/1


{function, gen_freq_table, 3, 15}.
  {label,14}.
    {line,[{location,"knucleotide.erl",31}]}.
    {func_info,{atom,knucleotide},{atom,gen_freq_table},3}.
  {label,15}.
    {'%',{var_info,{x,1},[{type,{t_bitstring,8,false}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,{0,288230376151711743}}}]}}.
    {line,[{location,"knucleotide.erl",32}]}.
    {gc_bif,byte_size,{f,0},3,[{tr,{x,1},{t_bitstring,8,false}}],{x,3}}.
    {gc_bif,'-',
            {f,0},
            4,
            [{tr,{x,3},{t_integer,{0,288230376151711743}}},
             {tr,{x,2},{t_integer,{0,288230376151711743}}}],
            {x,3}}.
    {swap,{x,2},{x,1}}.
    {swap,{x,0},{x,2}}.
    {call_only,4,{f,17}}. % gen_freq_table/4


{function, gen_freq_table, 4, 17}.
  {label,16}.
    {line,[{location,"knucleotide.erl",34}]}.
    {func_info,{atom,knucleotide},{atom,gen_freq_table},4}.
  {label,17}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,{0,288230376151711743}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_integer,any}}]}}.
    {test,is_eq_exact,{f,18},[{tr,{x,3},{t_integer,any}},{integer,-1}]}.
    {move,{atom,done},{x,0}}.
    return.
  {label,18}.
    {bs_start_match4,{atom,no_fail},4,{x,0},{x,4}}.
    {test,bs_skip_bits2,
          {f,19},
          [{tr,{x,4},{t_bs_context,8}},
           {tr,{x,3},{t_integer,any}},
           8,
           {field_flags,[{anno,[36,{file,"knucleotide.erl"}]},unsigned,big]}]}.
    {test,bs_get_binary2,
          {f,19},
          5,
          [{tr,{x,4},{t_bs_context,8}},
           {tr,{x,1},{t_integer,{0,288230376151711743}}},
           8,
           {field_flags,[{anno,[36,{file,"knucleotide.erl"}]},unsigned,big]}],
          {x,4}}.
    {allocate,4,5}.
    {move,{x,3},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,1},{y,2}}.
    {move,{x,0},{y,3}}.
    {move,{x,2},{x,1}}.
    {move,{x,4},{x,0}}.
    {line,[{location,"knucleotide.erl",37}]}.
    {call,2,{f,21}}. % update_counter/2
    {line,[{location,"knucleotide.erl",38}]}.
    {gc_bif,'-',{f,0},0,[{tr,{y,0},{t_integer,any}},{integer,1}],{x,3}}.
    {move,{y,1},{x,2}}.
    {move,{y,2},{x,1}}.
    {move,{y,3},{x,0}}.
    {call_last,4,{f,17},4}. % gen_freq_table/4
  {label,19}.
    {line,[{location,"knucleotide.erl",36}]}.
    {badmatch,{x,0}}.


{function, update_counter, 2, 21}.
  {label,20}.
    {line,[{location,"knucleotide.erl",41}]}.
    {func_info,{atom,knucleotide},{atom,update_counter},2}.
  {label,21}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {allocate,3,2}.
    {move,{x,1},{y,0}}.
    {move,{x,0},{y,1}}.
    {'try',{y,2},{f,22}}.
    {move,{integer,1},{x,2}}.
    {swap,{x,0},{x,1}}.
    {line,[{location,"knucleotide.erl",42}]}.
    {call_ext,3,{extfunc,ets,update_counter,3}}.
    {try_end,{y,2}}.
    {move,{atom,ok},{x,0}}.
    {deallocate,3}.
    return.
  {label,22}.
    {try_case,{y,2}}.
    {test,is_eq_exact,
          {f,23},
          [{tr,{x,0},{t_atom,[error,exit,throw]}},{atom,error}]}.
    {test,is_eq_exact,{f,23},[{x,1},{atom,badarg}]}.
    {test_heap,3,0}.
    {put_tuple2,{x,1},{list,[{y,1},{integer,1}]}}.
    {move,{y,0},{x,0}}.
    {line,[{scope,[0]},{location,"knucleotide.erl",43}]}.
    {call_ext_last,2,{extfunc,ets,insert,2},3}.
  {label,23}.
    {bif,raise,{f,0},[{x,2},{x,1}],{x,0}}.


{function, print_freq_table, 1, 25}.
  {label,24}.
    {line,[{location,"knucleotide.erl",47}]}.
    {func_info,{atom,knucleotide},{atom,print_freq_table},1}.
  {label,25}.
    {allocate_heap,1,{alloc,[{words,0},{floats,0},{funs,1}]},1}.
    {make_fun3,{f,57},0,0,{y,0},{list,[]}}.
    {'%',{var_info,{y,0},[{fun_type,{t_atom,[false,true]}}]}}.
    {line,[{location,"knucleotide.erl",48}]}.
    {call_ext,1,{extfunc,ets,tab2list,1}}.
    {move,{x,0},{x,1}}.
    {move,{y,0},{x,0}}.
    {init_yregs,{list,[{y,0}]}}.
    {call_ext,2,{extfunc,lists,sort,2}}.
    {test_heap,{alloc,[{words,0},{floats,0},{funs,1}]},1}.
    {make_fun3,{f,55},0,0,{x,1},{list,[]}}.
    {'%',{var_info,{x,1},[{fun_type,{t_number,any}}]}}.
    {move,{x,0},{y,0}}.
    {move,{x,0},{x,2}}.
    {move,{x,1},{x,0}}.
    {move,{integer,0},{x,1}}.
    {line,[{location,"knucleotide.erl",49}]}.
    {call_ext,3,{extfunc,lists,foldr,3}}.
    {test_heap,{alloc,[{words,1},{floats,0},{funs,1}]},1}.
    {make_fun3,{f,52},0,0,{x,0},{list,[{x,0}]}}.
    {move,{y,0},{x,1}}.
    {trim,1,0}.
    {line,[{location,"knucleotide.erl",50}]}.
    {call_ext,2,{extfunc,lists,foreach,2}}.
    {move,{literal,"\n"},{x,0}}.
    {line,[{location,"knucleotide.erl",53}]}.
    {call_ext_last,1,{extfunc,io,fwrite,1},0}.


{function, print_count, 2, 27}.
  {label,26}.
    {line,[{location,"knucleotide.erl",56}]}.
    {func_info,{atom,knucleotide},{atom,print_count},2}.
  {label,27}.
    {allocate,1,2}.
    {move,{x,1},{y,0}}.
    {line,[{location,"knucleotide.erl",57}]}.
    {call_ext,2,{extfunc,ets,lookup,2}}.
    {test,is_nonempty_list,{f,28},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_tuple,{f,29},[{x,1}]}.
    {test,test_arity,{f,29},[{x,1},2]}.
    {test,is_nil,{f,29},[{x,2}]}.
    {test_heap,4,2}.
    {get_tuple_element,{x,1},1,{x,0}}.
    {put_list,{y,0},nil,{x,1}}.
    {put_list,{x,0},{x,1},{x,1}}.
    {move,{literal,"~w\t~s\n"},{x,0}}.
    {line,[{location,"knucleotide.erl",58}]}.
    {call_ext_last,2,{extfunc,io,fwrite,2},1}.
  {label,28}.
    {test,is_nil,{f,29},[{x,0}]}.
    {test_heap,4,0}.
    {put_list,{y,0},nil,{x,0}}.
    {put_list,{integer,0},{x,0},{x,1}}.
    {move,{literal,"~w\t~s\n"},{x,0}}.
    {line,[{location,"knucleotide.erl",59}]}.
    {call_ext_last,2,{extfunc,io,fwrite,2},1}.
  {label,29}.
    {line,[{location,"knucleotide.erl",57}]}.
    {case_end,{x,0}}.


{function, do, 2, 31}.
  {label,30}.
    {line,[{location,"knucleotide.erl",63}]}.
    {func_info,{atom,knucleotide},{atom,do},2}.
  {label,31}.
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,2,true,
                                   #{1 => {t_fun,2,any,any},
                                     2 => {t_bitstring,8,false}}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_bitstring,8,false}}]}}.
    {test_heap,{alloc,[{words,3},{floats,0},{funs,1}]},2}.
    {get_tuple_element,{x,0},0,{x,2}}.
    {get_tuple_element,{x,0},1,{x,0}}.
    {make_fun3,{f,48},0,0,{x,0},{list,[{x,1},{x,0},{x,2}]}}.
    {line,[{location,"knucleotide.erl",64}]}.
    {call_ext_only,1,{extfunc,erlang,spawn,1}}.


{function, main, 1, 33}.
  {label,32}.
    {line,[{location,"knucleotide.erl",75}]}.
    {func_info,{atom,knucleotide},{atom,main},1}.
  {label,33}.
    {allocate,1,0}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{literal,[binary]},{x,1}}.
    {move,{atom,standard_io},{x,0}}.
    {line,[{location,"knucleotide.erl",76}]}.
    {call_ext,2,{extfunc,io,setopts,2}}.
    {line,[{location,"knucleotide.erl",77}]}.
    {call,0,{f,8}}. % seek_three/0
    {'%',{var_info,{x,0},[{type,{t_atom,[done]}}]}}.
    {move,nil,{x,0}}.
    {line,[{location,"knucleotide.erl",78}]}.
    {call,1,{f,12}}. % get_seq_three/1
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {test_heap,{alloc,[{words,35},{floats,0},{funs,2}]},1}.
    {make_fun3,{f,46},0,0,{x,1},{list,[]}}.
    {make_fun3,{f,44},0,0,{x,2},{list,[]}}.
    {put_tuple2,{x,3},{list,[{x,2},{literal,<<"GGTATTTTAATTTATAGT">>}]}}.
    {put_list,{x,3},nil,{x,3}}.
    {put_tuple2,{x,4},{list,[{x,2},{literal,<<"GGTATTTTAATT">>}]}}.
    {put_list,{x,4},{x,3},{x,3}}.
    {put_tuple2,{x,4},{list,[{x,2},{literal,<<"GGTATT">>}]}}.
    {put_list,{x,4},{x,3},{x,3}}.
    {put_tuple2,{x,4},{list,[{x,2},{literal,<<"GGTA">>}]}}.
    {put_list,{x,4},{x,3},{x,3}}.
    {put_tuple2,{x,2},{list,[{x,2},{literal,<<"GGT">>}]}}.
    {put_list,{x,2},{x,3},{x,2}}.
    {put_tuple2,{x,3},{list,[{x,1},{literal,<<"??">>}]}}.
    {put_list,{x,3},{x,2},{x,2}}.
    {put_tuple2,{x,1},{list,[{x,1},{literal,<<"?">>}]}}.
    {put_list,{x,1},{x,2},{x,1}}.
    {swap,{x,0},{x,1}}.
    {line,[{location,"knucleotide.erl",89}]}.
    {call,2,{f,41}}. % '-main/1-lc$^2/1-0-'/2
    {'%',{var_info,{x,0},[{type,{t_cons,pid,nil}}]}}.
    {test_heap,2,1}.
    {get_list,{x,0},{y,0},{x,0}}.
    {bif,self,{f,0},[],{x,1}}.
    {put_list,{x,1},nil,{x,1}}.
    {line,[{location,"knucleotide.erl",91}]}.
    {call_ext,2,{extfunc,erlang,'++',2}}.
    {move,{x,0},{x,1}}.
    {move,{y,0},{x,0}}.
    {init_yregs,{list,[{y,0}]}}.
    send.
  {label,34}.
    {loop_rec,{f,35},{x,0}}.
    remove_message.
    {move,{integer,0},{x,0}}.
    {line,[{location,"knucleotide.erl",92}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},1}.
  {label,35}.
    {wait,{f,34}}.


{function, module_info, 0, 37}.
  {label,36}.
    {line,[]}.
    {func_info,{atom,knucleotide},{atom,module_info},0}.
  {label,37}.
    {move,{atom,knucleotide},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 39}.
  {label,38}.
    {line,[]}.
    {func_info,{atom,knucleotide},{atom,module_info},1}.
  {label,39}.
    {move,{x,0},{x,1}}.
    {move,{atom,knucleotide},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-main/1-lc$^2/1-0-', 2, 41}.
  {label,40}.
    {line,[{location,"knucleotide.erl",89}]}.
    {func_info,{atom,knucleotide},{atom,'-main/1-lc$^2/1-0-'},2}.
  {label,41}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => {t_fun,2,any,any},2 => {t_bitstring,8,false}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_bitstring,8,false}}]}}.
    {test,is_nonempty_list,{f,42},[{x,0}]}.
    {allocate,2,2}.
    {move,{x,1},{y,1}}.
    {get_list,{x,0},{x,0},{y,0}}.
    {call,2,{f,31}}. % do/2
    {'%',{var_info,{x,0},[{type,pid}]}}.
    {move,{y,0},{x,2}}.
    {move,{y,1},{x,1}}.
    {move,{x,0},{y,1}}.
    {trim,1,1}.
    {move,{x,2},{x,0}}.
    {call,2,{f,41}}. % '-main/1-lc$^2/1-0-'/2
    {'%',{var_info,{x,0},[{type,{t_list,pid,nil}}]}}.
    {test_heap,2,1}.
    {put_list,{y,0},{x,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,42}.
    return.


{function, '-main/1-fun-1-', 2, 44}.
  {label,43}.
    {line,[{location,"knucleotide.erl",80}]}.
    {func_info,{atom,knucleotide},{atom,'-main/1-fun-1-'},2}.
  {label,44}.
    {call_only,2,{f,27}}. % print_count/2


{function, '-main/1-fun-0-', 2, 46}.
  {label,45}.
    {line,[{location,"knucleotide.erl",79}]}.
    {func_info,{atom,knucleotide},{atom,'-main/1-fun-0-'},2}.
  {label,46}.
    {call_only,1,{f,25}}. % print_freq_table/1


{function, '-do/2-fun-0-', 3, 48}.
  {label,47}.
    {line,[{location,"knucleotide.erl",64}]}.
    {func_info,{atom,knucleotide},{atom,'-do/2-fun-0-'},3}.
  {label,48}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {'%',{var_info,{x,1},[{type,{t_bitstring,8,false}}]}}.
    {'%',{var_info,{x,2},[{type,{t_fun,2,any,any}}]}}.
    {allocate,4,3}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,2},{y,1}}.
    {move,{x,1},{y,2}}.
    {move,{x,0},{y,3}}.
    {move,{literal,[set]},{x,1}}.
    {move,{atom,hash},{x,0}}.
    {line,[{location,"knucleotide.erl",65}]}.
    {call_ext,2,{extfunc,ets,new,2}}.
    {line,[{location,"knucleotide.erl",66}]}.
    {gc_bif,byte_size,{f,0},1,[{tr,{y,2},{t_bitstring,8,false}}],{x,2}}.
    {move,{y,3},{x,1}}.
    {move,{x,0},{y,3}}.
    {call,3,{f,15}}. % gen_freq_table/3
    {'%',{var_info,{x,0},[{type,{t_atom,[done]}}]}}.
  {label,49}.
    {loop_rec,{f,50},{x,0}}.
    {move,{x,0},{y,0}}.
    remove_message.
    {move,{y,1},{x,2}}.
    {move,{y,2},{x,1}}.
    {move,{y,0},{y,2}}.
    {trim,2,2}.
    {move,{y,1},{x,0}}.
    {line,[{location,"knucleotide.erl",69}]}.
    {call_fun2,{atom,safe},2,{tr,{x,2},{t_fun,2,any,any}}}.
    {line,[{location,"knucleotide.erl",70}]}.
    {bif,hd,{f,0},[{y,0}],{x,0}}.
    {get_tl,{y,0},{x,1}}.
    {trim,1,1}.
    send.
    {move,{y,0},{x,0}}.
    {line,[{location,"knucleotide.erl",72}]}.
    {call_ext_last,1,{extfunc,ets,delete,1},1}.
  {label,50}.
    {wait,{f,49}}.


{function, '-print_freq_table/1-fun-2-', 2, 52}.
  {label,51}.
    {line,[{location,"knucleotide.erl",50}]}.
    {func_info,{atom,knucleotide},{atom,'-print_freq_table/1-fun-2-'},2}.
  {label,52}.
    {'%',{var_info,{x,1},[{type,{t_number,any}}]}}.
    {test,is_tuple,{f,53},[{x,0}]}.
    {test,test_arity,{f,53},[{x,0},2]}.
    {get_tuple_element,{x,0},1,{x,2}}.
    {line,[{location,"knucleotide.erl",51}]}.
    {gc_bif,'*',{f,0},3,[{x,2},{integer,100}],{x,2}}.
    {fconv,{tr,{x,2},{t_number,any}},{fr,0}}.
    {fconv,{tr,{x,1},{t_number,any}},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,4},{floats,1},{funs,0}]},1}.
    {fmove,{fr,0},{x,1}}.
    {get_tuple_element,{x,0},0,{x,0}}.
    {put_list,{x,1},nil,{x,1}}.
    {put_list,{x,0},{x,1},{x,1}}.
    {move,{literal,"~s ~.3f\n"},{x,0}}.
    {call_ext_only,2,{extfunc,io,fwrite,2}}.
  {label,53}.
    {call_only,1,{f,61}}. % '-print_freq_table/1-inlined-2-'/1


{function, '-print_freq_table/1-fun-1-', 2, 55}.
  {label,54}.
    {line,[{location,"knucleotide.erl",49}]}.
    {func_info,{atom,knucleotide},{atom,'-print_freq_table/1-fun-1-'},2}.
  {label,55}.
    {test,is_tuple,{f,54},[{x,0}]}.
    {test,test_arity,{f,54},[{x,0},2]}.
    {get_tuple_element,{x,0},1,{x,0}}.
    {gc_bif,'+',{f,0},2,[{x,1},{x,0}],{x,0}}.
    return.


{function, '-print_freq_table/1-fun-0-', 2, 57}.
  {label,56}.
    {line,[{location,"knucleotide.erl",48}]}.
    {func_info,{atom,knucleotide},{atom,'-print_freq_table/1-fun-0-'},2}.
  {label,57}.
    {test,is_tuple,{f,56},[{x,0}]}.
    {test,test_arity,{f,56},[{x,0},2]}.
    {test,is_tuple,{f,56},[{x,1}]}.
    {test,test_arity,{f,56},[{x,1},2]}.
    {get_tuple_element,{x,0},1,{x,2}}.
    {get_tuple_element,{x,1},1,{x,3}}.
    {test,is_lt,{f,58},[{x,3},{x,2}]}.
    {move,{atom,true},{x,0}}.
    return.
  {label,58}.
    {test,is_eq,{f,59},[{x,2},{x,3}]}.
    {get_tuple_element,{x,1},0,{x,1}}.
    {get_tuple_element,{x,0},0,{x,0}}.
    {bif,'<',{f,0},[{x,0},{x,1}],{x,0}}.
    return.
  {label,59}.
    {move,{atom,false},{x,0}}.
    return.


{function, '-print_freq_table/1-inlined-2-', 1, 61}.
  {label,60}.
    {line,[{location,"knucleotide.erl",50}]}.
    {func_info,{atom,knucleotide},{atom,'-print_freq_table/1-inlined-2-'},1}.
  {label,61}.
    {jump,{f,60}}.
