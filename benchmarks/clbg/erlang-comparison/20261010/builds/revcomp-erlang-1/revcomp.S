{module, revcomp}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 97}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"revcomp.erl",20}]}.
    {func_info,{atom,revcomp},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,1},[{x,0}]}.
    {get_tl,{x,0},{x,1}}.
    {test,is_nil,{f,1},[{x,1}]}.
    {allocate,0,0}.
    {move,{literal,[binary]},{x,0}}.
    {line,[{location,"revcomp.erl",21}]}.
    {call_ext,1,{extfunc,io,setopts,1}}.
    {line,[{location,"revcomp.erl",22}]}.
    {call,0,{f,4}}. % run_parallel/0
    {'%',{var_info,{x,0},[{type,{t_atom,[ok]}}]}}.
    {line,[{location,"revcomp.erl",23}]}.
    {call_ext_last,0,{extfunc,erlang,halt,0},0}.


{function, run_parallel, 0, 4}.
  {label,3}.
    {line,[{location,"revcomp.erl",27}]}.
    {func_info,{atom,revcomp},{atom,run_parallel},0}.
  {label,4}.
    {allocate,0,0}.
    {bif,self,{f,0},[],{x,1}}.
    {move,{atom,reader},{x,0}}.
    {line,[{location,"revcomp.erl",28}]}.
    {call_ext,2,{extfunc,erlang,register,2}}.
    {move,{atom,go},{x,1}}.
    {move,{atom,reader},{x,0}}.
    {line,[{location,"revcomp.erl",29}]}.
    send.
    {move,{integer,256},{x,0}}.
    bs_init_writable.
    {call_last,1,{f,6},0}. % loop/1


{function, loop, 1, 6}.
  {label,5}.
    {line,[{location,"revcomp.erl",32}]}.
    {func_info,{atom,revcomp},{atom,loop},1}.
  {label,6}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {allocate,2,1}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,0},{y,1}}.
    {line,[{location,"revcomp.erl",33}]}.
    {call,0,{f,19}}. % get_line/0
    {move,{x,0},{y,0}}.
    {test,bs_start_match3,{f,10},1,[{x,0}],{x,0}}.
    {bs_match,{f,17},
              {x,0},
              {commands,[{ensure_at_least,8,8},{'=:=',nil,8,62}]}}.
  {label,7}.
    {loop_rec,{f,9},{x,0}}.
    {test,is_eq_exact,{f,8},[{x,0},{atom,go}]}.
    {test_heap,{alloc,[{words,2},{floats,0},{funs,1}]},0}.
    remove_message.
    {make_fun3,{f,94},0,0,{x,0},{list,[{y,1},{y,0}]}}.
    {init_yregs,{list,[{y,0},{y,1}]}}.
    {line,[{location,"revcomp.erl",40}]}.
    {call_ext,1,{extfunc,erlang,spawn,1}}.
    {move,{integer,256},{x,0}}.
    bs_init_writable.
    {call_last,1,{f,6},2}. % loop/1
  {label,8}.
    {loop_rec_end,{f,7}}.
  {label,9}.
    {wait,{f,7}}.
  {label,10}.
    {test,is_eq_exact,{f,17},[{y,0},{atom,eof}]}.
  {label,11}.
    {loop_rec,{f,16},{x,0}}.
    {test,is_eq_exact,{f,15},[{x,0},{atom,go}]}.
    {test_heap,{alloc,[{words,1},{floats,0},{funs,1}]},0}.
    remove_message.
    {make_fun3,{f,96},0,0,{x,0},{list,[{y,1}]}}.
    {init_yregs,{list,[{y,0},{y,1}]}}.
    {line,[{location,"revcomp.erl",36}]}.
    {call_ext,1,{extfunc,erlang,spawn,1}}.
  {label,12}.
    {loop_rec,{f,14},{x,0}}.
    {test,is_eq_exact,{f,13},[{x,0},{atom,go}]}.
    remove_message.
    {move,{atom,ok},{x,0}}.
    {deallocate,2}.
    return.
  {label,13}.
    {loop_rec_end,{f,12}}.
  {label,14}.
    {wait,{f,12}}.
  {label,15}.
    {loop_rec_end,{f,11}}.
  {label,16}.
    {wait,{f,11}}.
  {label,17}.
    {line,[{location,"revcomp.erl",46}]}.
    {bs_create_bin,{f,0},
                   0,0,8,
                   {x,0},
                   {list,[{atom,private_append},
                          1,8,nil,
                          {tr,{y,1},{t_bitstring,8,true}},
                          {atom,all},
                          {atom,binary},
                          2,8,nil,
                          {y,0},
                          {atom,all}]}}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {call_last,1,{f,6},2}. % loop/1


{function, get_line, 0, 19}.
  {label,18}.
    {line,[{location,"revcomp.erl",49}]}.
    {func_info,{atom,revcomp},{atom,get_line},0}.
  {label,19}.
    {allocate,2,0}.
    {init_yregs,{list,[{y,0},{y,1}]}}.
    {bif,get,{f,0},[{atom,linebuf}],{x,0}}.
    {test,is_eq_exact,{f,20},[{x,0},{atom,undefined}]}.
    {move,{literal,<<>>},{y,1}}.
    {jump,{f,21}}.
  {label,20}.
    {move,{x,0},{y,1}}.
  {label,21}.
    {move,{literal,<<"\n">>},{x,1}}.
    {move,{y,1},{x,0}}.
    {line,[{location,"revcomp.erl",54}]}.
    {call_ext,2,{extfunc,binary,split,2}}.
    {test,is_nonempty_list,{f,26},[{x,0}]}.
    {get_list,{x,0},{y,0},{x,1}}.
    {test,is_nonempty_list,{f,22},[{x,1}]}.
    {get_list,{x,1},{x,2},{x,1}}.
    {test,is_nil,{f,26},[{x,1}]}.
    {move,{x,2},{x,1}}.
    {move,{y,0},{y,1}}.
    {trim,1,1}.
    {move,{atom,linebuf},{x,0}}.
    {line,[{location,"revcomp.erl",56}]}.
    {call_ext,2,{extfunc,erlang,put,2}}.
    {move,{y,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,22}.
    {test,is_nil,{f,26},[{x,1}]}.
    {bif,get,{f,0},[{atom,linebuf_eof}],{x,0}}.
    {bif,'=/=',{f,0},[{x,0},{atom,undefined}],{x,0}}.
    {test,is_eq_exact,{f,23},[{y,1},{literal,<<>>}]}.
    {test,is_eq_exact,{f,23},[{x,0},{atom,true}]}.
    {move,{atom,eof},{x,0}}.
    {deallocate,2}.
    return.
  {label,23}.
    {move,{integer,4096},{x,1}}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{atom,standard_io},{x,0}}.
    {line,[{location,"revcomp.erl",63}]}.
    {call_ext,2,{extfunc,file,read,2}}.
    {test,is_tuple,{f,24},[{x,0}]}.
    {test,is_tagged_tuple,
          {f,25},
          [{tr,{x,0},{t_tuple,0,false,#{}}},2,{atom,ok}]}.
    {get_tuple_element,{x,0},1,{x,0}}.
    {line,[{location,"revcomp.erl",68}]}.
    {bs_create_bin,{f,0},
                   0,1,8,
                   {x,1},
                   {list,[{atom,append},
                          1,8,nil,
                          {y,1},
                          {atom,all},
                          {atom,binary},
                          2,8,nil,
                          {x,0},
                          {atom,all}]}}.
    {trim,2,0}.
    {move,{atom,linebuf},{x,0}}.
    {call_ext,2,{extfunc,erlang,put,2}}.
    {call_last,0,{f,19},0}. % get_line/0
  {label,24}.
    {test,is_eq_exact,{f,25},[{x,0},{atom,eof}]}.
    {move,{atom,true},{x,1}}.
    {trim,2,0}.
    {move,{atom,linebuf_eof},{x,0}}.
    {line,[{location,"revcomp.erl",65}]}.
    {call_ext,2,{extfunc,erlang,put,2}}.
    {call_last,0,{f,19},0}. % get_line/0
  {label,25}.
    {line,[{location,"revcomp.erl",63}]}.
    {case_end,{x,0}}.
  {label,26}.
    {line,[{location,"revcomp.erl",54}]}.
    {case_end,{x,0}}.


{function, flush, 2, 28}.
  {label,27}.
    {line,[{location,"revcomp.erl",79}]}.
    {func_info,{atom,revcomp},{atom,flush},2}.
  {label,28}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_union,none,
                     {t_cons,
                         {t_union,none,none,
                             {t_integer,{10,10}},
                             none,none,
                             {t_bs_matchable,1}},
                         nil},
                     none,none,none,
                     {t_bitstring,256,true}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_bitstring,8,true}}]}}.
    {allocate,2,2}.
    {move,{x,1},{y,0}}.
    {move,{x,0},{y,1}}.
    {bif,self,{f,0},[],{x,1}}.
    {move,{atom,collector},{x,0}}.
    {line,[{location,"revcomp.erl",80}]}.
    {call_ext,2,{extfunc,erlang,register,2}}.
    {move,{y,0},{x,0}}.
    {trim,1,1}.
    {line,[{location,"revcomp.erl",81}]}.
    {call,1,{f,30}}. % reverse_complement/1
    {'%',{var_info,{x,0},
                   [{type,{t_union,none,
                                   {t_list,any,nil},
                                   none,none,none,
                                   {t_bitstring,256,true}}}]}}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {move,{y,0},{x,0}}.
    {trim,1,0}.
    {line,[{location,"revcomp.erl",82}]}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {move,{atom,collector},{x,0}}.
    {line,[{location,"revcomp.erl",83}]}.
    {call_ext,1,{extfunc,erlang,unregister,1}}.
    {move,{atom,go},{x,1}}.
    {move,{atom,reader},{x,0}}.
    {line,[{location,"revcomp.erl",84}]}.
    send.
    {deallocate,0}.
    return.


{function, reverse_complement, 1, 30}.
  {label,29}.
    {line,[{location,"revcomp.erl",90}]}.
    {func_info,{atom,revcomp},{atom,reverse_complement},1}.
  {label,30}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {test,is_eq_exact,{f,31},[{tr,{x,0},{t_bitstring,8,true}},{literal,<<>>}]}.
    return.
  {label,31}.
    {allocate,3,1}.
    {init_yregs,{list,[{y,0},{y,1}]}}.
    {move,{x,0},{y,2}}.
    {line,[{location,"revcomp.erl",93}]}.
    {gc_bif,byte_size,{f,0},1,[{tr,{x,0},{t_bitstring,8,true}}],{x,0}}.
    {move,{integer,4},{x,1}}.
    {call,2,{f,51}}. % calculate_splits/2
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,2,true,
                                   #{1 => {t_integer,any},
                                     2 => {t_integer,{0,'+inf'}}}}}]}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{y,1}}.
    {move,{y,2},{x,0}}.
    {line,[{location,"revcomp.erl",94}]}.
    {call,2,{f,33}}. % start_jobs/2
    {'%',{var_info,{x,0},[{type,{t_integer,{0,4}}}]}}.
    {move,{x,0},{y,0}}.
    {move,{integer,256},{x,0}}.
    bs_init_writable.
    {move,{x,0},{x,2}}.
    {move,{y,1},{x,1}}.
    {move,{y,2},{x,0}}.
    {init_yregs,{list,[{y,1},{y,2}]}}.
    {line,[{location,"revcomp.erl",95}]}.
    {call,3,{f,43}}. % revcomp_last/3
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {move,{x,0},{y,2}}.
    {move,{y,0},{x,0}}.
    {trim,2,1}.
    {line,[{location,"revcomp.erl",96}]}.
    {call,1,{f,72}}. % collect/1
    {'%',{var_info,{x,0},[{type,{t_list,any,nil}}]}}.
    {test_heap,2,1}.
    {put_list,{y,0},nil,{x,1}}.
    {call_ext_last,2,{extfunc,erlang,'++',2},1}.


{function, start_jobs, 2, 33}.
  {label,32}.
    {line,[{location,"revcomp.erl",98}]}.
    {func_info,{atom,revcomp},{atom,start_jobs},2}.
  {label,33}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {test,is_eq_exact,{f,34},[{tr,{x,1},{t_integer,any}},{integer,0}]}.
    {move,{integer,0},{x,0}}.
    return.
  {label,34}.
    {line,[{location,"revcomp.erl",101}]}.
    {gc_bif,byte_size,{f,0},2,[{tr,{x,0},{t_bitstring,8,true}}],{x,2}}.
    {move,{integer,0},{x,3}}.
    {call_only,4,{f,36}}. % start_jobs/4


{function, start_jobs, 4, 36}.
  {label,35}.
    {line,[{location,"revcomp.erl",103}]}.
    {func_info,{atom,revcomp},{atom,start_jobs},4}.
  {label,36}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,3},[{type,{t_integer,{0,'+inf'}}}]}}.
    {test,is_eq_exact,{f,37},[{tr,{x,3},{t_integer,{0,'+inf'}}},{integer,4}]}.
    {move,{x,3},{x,0}}.
    return.
  {label,37}.
    {test,is_ge,
          {f,35},
          [{tr,{x,2},{t_integer,any}},{tr,{x,1},{t_integer,any}}]}.
    {test_heap,{alloc,[{words,0},{floats,0},{funs,1}]},4}.
    {make_fun3,{f,39},0,0,{x,4},{list,[]}}.
    {line,[{location,"revcomp.erl",106}]}.
    {gc_bif,'-',
            {f,0},
            5,
            [{tr,{x,2},{t_integer,any}},{tr,{x,1},{t_integer,any}}],
            {x,5}}.
    {allocate_heap,4,9,6}.
    {move,{x,3},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,1},{y,2}}.
    {move,{x,0},{y,3}}.
    {put_list,{x,2},{literal,[<<>>]},{x,0}}.
    {put_list,{x,5},{x,0},{x,0}}.
    {put_list,{y,3},{x,0},{x,0}}.
    {put_tuple2,{x,0},{list,[{x,4},{x,0}]}}.
    {move,{x,3},{x,1}}.
    {call,2,{f,70}}. % new_job/2
    {'%',{var_info,{x,0},[{type,pid}]}}.
    {line,[{location,"revcomp.erl",107}]}.
    {gc_bif,'-',
            {f,0},
            0,
            [{tr,{y,1},{t_integer,any}},{tr,{y,2},{t_integer,any}}],
            {x,0}}.
    {gc_bif,'+',{f,0},1,[{tr,{y,0},{t_integer,{0,'+inf'}}},{integer,1}],{x,3}}.
    {move,{y,2},{x,1}}.
    {move,{x,0},{x,2}}.
    {move,{y,3},{x,0}}.
    {call_last,4,{f,36},4}. % start_jobs/4


{function, revcomp_chunk, 4, 39}.
  {label,38}.
    {line,[{location,"revcomp.erl",110}]}.
    {func_info,{atom,revcomp},{atom,revcomp_chunk},4}.
  {label,39}.
    {test,is_eq_exact,{f,40},[{x,2},{x,1}]}.
    {move,{x,3},{x,0}}.
    return.
  {label,40}.
    {line,[{location,"revcomp.erl",113}]}.
    {gc_bif,'-',{f,0},4,[{x,2},{integer,60}],{x,2}}.
    {test,bs_start_match3,{f,41},4,[{x,0}],{x,4}}.
    {test,bs_skip_bits2,
          {f,41},
          [{x,4},
           {tr,{x,2},{t_number,any}},
           8,
           {field_flags,[{anno,[114,{file,"revcomp.erl"}]},unsigned,big]}]}.
    {bs_match,{f,41},
              {x,4},
              {commands,[{ensure_at_least,480,8},
                         {binary,5,{literal,[]},60,8,{x,5}}]}}.
    {allocate,4,6}.
    {move,{x,2},{y,0}}.
    {move,{x,3},{y,1}}.
    {move,{x,1},{y,2}}.
    {move,{x,0},{y,3}}.
    {move,{x,5},{x,0}}.
    {line,[{location,"revcomp.erl",115}]}.
    {call,1,{f,48}}. % revcomp/1
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {line,[{location,"revcomp.erl",116}]}.
    {bs_create_bin,{f,0},
                   0,1,8,
                   {x,3},
                   {list,[{atom,append},
                          1,8,nil,
                          {y,1},
                          {atom,all},
                          {atom,binary},
                          2,8,nil,
                          {tr,{x,0},{t_bitstring,8,false}},
                          {atom,all}]}}.
    {move,{y,0},{x,2}}.
    {move,{y,2},{x,1}}.
    {move,{y,3},{x,0}}.
    {call_last,4,{f,39},4}. % revcomp_chunk/4
  {label,41}.
    {line,[{location,"revcomp.erl",114}]}.
    {badmatch,{x,0}}.


{function, revcomp_last, 3, 43}.
  {label,42}.
    {line,[{location,"revcomp.erl",119}]}.
    {func_info,{atom,revcomp},{atom,revcomp_last},3}.
  {label,43}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_bitstring,8,true}}]}}.
    {test,is_ge,{f,44},[{tr,{x,1},{t_integer,any}},{integer,61}]}.
    {line,[{location,"revcomp.erl",120}]}.
    {gc_bif,'-',
            {f,0},
            3,
            [{tr,{x,1},{t_integer,{61,'+inf'}}},{integer,60}],
            {x,1}}.
    {bs_start_match4,{atom,no_fail},3,{x,0},{x,3}}.
    {test,bs_skip_bits2,
          {f,45},
          [{x,3},
           {tr,{x,1},{t_integer,{1,'+inf'}}},
           8,
           {field_flags,[{anno,[121,{file,"revcomp.erl"}]},unsigned,big]}]}.
    {bs_match,{f,45},
              {x,3},
              {commands,[{ensure_at_least,480,1},
                         {binary,4,{literal,[]},60,8,{x,3}}]}}.
    {allocate,3,4}.
    {move,{x,1},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,0},{y,2}}.
    {move,{x,3},{x,0}}.
    {line,[{location,"revcomp.erl",122}]}.
    {call,1,{f,48}}. % revcomp/1
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {line,[{location,"revcomp.erl",123}]}.
    {bs_create_bin,{f,0},
                   0,1,8,
                   {x,2},
                   {list,[{atom,private_append},
                          1,8,nil,
                          {tr,{y,1},{t_bitstring,8,true}},
                          {atom,all},
                          {atom,binary},
                          2,8,nil,
                          {tr,{x,0},{t_bitstring,8,false}},
                          {atom,all}]}}.
    {'%',{var_info,{x,2},[{type,{t_bitstring,8,true}}]}}.
    {move,{y,0},{x,1}}.
    {move,{y,2},{x,0}}.
    {call_last,3,{f,43},3}. % revcomp_last/3
  {label,44}.
    {bs_start_match4,{atom,no_fail},3,{x,0},{x,3}}.
    {test,bs_get_binary2,
          {f,46},
          4,
          [{tr,{x,3},{t_bs_context,8}},
           {tr,{x,1},{t_integer,{'-inf',60}}},
           8,
           {field_flags,[{anno,[125,{file,"revcomp.erl"}]},unsigned,big]}],
          {x,1}}.
    {allocate,1,3}.
    {move,{x,2},{y,0}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"revcomp.erl",126}]}.
    {call,1,{f,48}}. % revcomp/1
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {line,[{location,"revcomp.erl",127}]}.
    {bs_create_bin,{f,0},
                   0,1,8,
                   {x,0},
                   {list,[{atom,private_append},
                          1,8,nil,
                          {tr,{y,0},{t_bitstring,8,true}},
                          {atom,all},
                          {atom,binary},
                          2,8,nil,
                          {tr,{x,0},{t_bitstring,8,false}},
                          {atom,all}]}}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {deallocate,1}.
    return.
  {label,45}.
    {line,[{location,"revcomp.erl",121}]}.
    {badmatch,{x,0}}.
  {label,46}.
    {line,[{location,"revcomp.erl",125}]}.
    {badmatch,{x,0}}.


{function, revcomp, 1, 48}.
  {label,47}.
    {line,[{location,"revcomp.erl",131}]}.
    {func_info,{atom,revcomp},{atom,revcomp},1}.
  {label,48}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {test,is_eq_exact,
          {f,49},
          [{tr,{x,0},{t_bitstring,8,false}},{literal,<<>>}]}.
    return.
  {label,49}.
    {allocate,0,1}.
    {line,[{location,"revcomp.erl",135}]}.
    {call_ext,1,{extfunc,erlang,binary_to_list,1}}.
    {call,1,{f,91}}. % '-revcomp/1-lc$^0/1-0-'/1
    {'%',{var_info,{x,0},[{type,{t_list,{t_integer,{65,89}},nil}}]}}.
    {test_heap,2,1}.
    {put_list,{integer,10},{x,0},{x,0}}.
    {line,[{location,"revcomp.erl",134}]}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {call_ext_last,1,{extfunc,erlang,list_to_binary,1},0}.


{function, calculate_splits, 2, 51}.
  {label,50}.
    {line,[{location,"revcomp.erl",137}]}.
    {func_info,{atom,revcomp},{atom,calculate_splits},2}.
  {label,51}.
    {'%',{var_info,{x,0},[{type,{t_integer,{0,288230376151711743}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,{4,4}}}]}}.
    {line,[{location,"revcomp.erl",138}]}.
    {gc_bif,'div',
            {f,0},
            1,
            [{tr,{x,0},{t_integer,{0,288230376151711743}}},{integer,4}],
            {x,1}}.
    {line,[{location,"revcomp.erl",139}]}.
    {gc_bif,'rem',
            {f,0},
            2,
            [{tr,{x,1},{t_integer,{0,72057594037927935}}},{integer,60}],
            {x,2}}.
    {line,[{location,"revcomp.erl",140}]}.
    {gc_bif,'-',
            {f,0},
            3,
            [{tr,{x,1},{t_integer,{0,72057594037927935}}},
             {tr,{x,2},{t_integer,{0,59}}}],
            {x,1}}.
    {line,[{location,"revcomp.erl",141}]}.
    {gc_bif,'rem',
            {f,0},
            3,
            [{tr,{x,0},{t_integer,{0,288230376151711743}}},{integer,4}],
            {x,0}}.
    {gc_bif,'*',{f,0},3,[{tr,{x,2},{t_integer,{0,59}}},{integer,4}],{x,2}}.
    {gc_bif,'+',
            {f,0},
            3,
            [{tr,{x,0},{t_integer,{0,3}}},{tr,{x,2},{t_integer,{0,236}}}],
            {x,0}}.
    {test_heap,3,2}.
    {put_tuple2,{x,0},{list,[{x,1},{x,0}]}}.
    return.


{function, complement, 1, 53}.
  {label,52}.
    {line,[{location,"revcomp.erl",144}]}.
    {func_info,{atom,revcomp},{atom,complement},1}.
  {label,53}.
    {select_val,{x,0},
                {f,52},
                {list,[{integer,65},
                       {f,68},
                       {integer,66},
                       {f,67},
                       {integer,67},
                       {f,66},
                       {integer,68},
                       {f,65},
                       {integer,71},
                       {f,64},
                       {integer,72},
                       {f,63},
                       {integer,75},
                       {f,62},
                       {integer,77},
                       {f,61},
                       {integer,78},
                       {f,60},
                       {integer,82},
                       {f,59},
                       {integer,83},
                       {f,58},
                       {integer,84},
                       {f,57},
                       {integer,85},
                       {f,57},
                       {integer,86},
                       {f,56},
                       {integer,87},
                       {f,55},
                       {integer,89},
                       {f,54},
                       {integer,97},
                       {f,68},
                       {integer,98},
                       {f,67},
                       {integer,99},
                       {f,66},
                       {integer,100},
                       {f,65},
                       {integer,103},
                       {f,64},
                       {integer,104},
                       {f,63},
                       {integer,107},
                       {f,62},
                       {integer,109},
                       {f,61},
                       {integer,110},
                       {f,60},
                       {integer,114},
                       {f,59},
                       {integer,115},
                       {f,58},
                       {integer,116},
                       {f,57},
                       {integer,117},
                       {f,57},
                       {integer,118},
                       {f,56},
                       {integer,119},
                       {f,55},
                       {integer,121},
                       {f,54}]}}.
  {label,54}.
    {move,{integer,82},{x,0}}.
    return.
  {label,55}.
    {move,{integer,87},{x,0}}.
    return.
  {label,56}.
    {move,{integer,66},{x,0}}.
    return.
  {label,57}.
    {move,{integer,65},{x,0}}.
    return.
  {label,58}.
    {move,{integer,83},{x,0}}.
    return.
  {label,59}.
    {move,{integer,89},{x,0}}.
    return.
  {label,60}.
    {move,{integer,78},{x,0}}.
    return.
  {label,61}.
    {move,{integer,75},{x,0}}.
    return.
  {label,62}.
    {move,{integer,77},{x,0}}.
    return.
  {label,63}.
    {move,{integer,68},{x,0}}.
    return.
  {label,64}.
    {move,{integer,67},{x,0}}.
    return.
  {label,65}.
    {move,{integer,72},{x,0}}.
    return.
  {label,66}.
    {move,{integer,71},{x,0}}.
    return.
  {label,67}.
    {move,{integer,86},{x,0}}.
    return.
  {label,68}.
    {move,{integer,84},{x,0}}.
    return.


{function, new_job, 2, 70}.
  {label,69}.
    {line,[{location,"revcomp.erl",178}]}.
    {func_info,{atom,revcomp},{atom,new_job},2}.
  {label,70}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_tuple,2,true,
                     #{1 => {t_fun,4,{revcomp_chunk,4},any},
                       2 =>
                           {t_cons,
                               {t_union,none,none,
                                   {t_integer,any},
                                   none,none,
                                   {t_bitstring,8,true}},
                               nil}}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,{0,'+inf'}}}]}}.
    {test_heap,{alloc,[{words,3},{floats,0},{funs,1}]},2}.
    {get_tuple_element,{x,0},0,{x,2}}.
    {get_tuple_element,{x,0},1,{x,0}}.
    {make_fun3,{f,89},0,0,{x,0},{list,[{x,1},{x,0},{x,2}]}}.
    {line,[{location,"revcomp.erl",179}]}.
    {call_ext_only,1,{extfunc,erlang,spawn,1}}.


{function, collect, 1, 72}.
  {label,71}.
    {line,[{location,"revcomp.erl",181}]}.
    {func_info,{atom,revcomp},{atom,collect},1}.
  {label,72}.
    {'%',{var_info,{x,0},[{type,{t_integer,{0,4}}}]}}.
    {move,nil,{x,1}}.
    {call_only,2,{f,74}}. % collect/2


{function, collect, 2, 74}.
  {label,73}.
    {line,[{location,"revcomp.erl",182}]}.
    {func_info,{atom,revcomp},{atom,collect},2}.
  {label,74}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_list,{t_tuple,2,true,#{}},nil}}]}}.
    {test,is_eq_exact,{f,75},[{tr,{x,0},{t_integer,any}},{integer,0}]}.
    {allocate,0,2}.
    {move,{integer,1},{x,0}}.
    {call_ext,2,{extfunc,lists,keysort,2}}.
    {call_last,1,{f,84},0}. % '-collect/2-lc$^0/1-0-'/1
  {label,75}.
    {allocate,2,2}.
    {move,{x,1},{y,0}}.
    {move,{x,0},{y,1}}.
  {label,76}.
    {loop_rec,{f,78},{x,0}}.
    {test,is_tuple,{f,77},[{x,0}]}.
    {test,test_arity,{f,77},[{x,0},2]}.
    remove_message.
    {line,[{location,"revcomp.erl",183}]}.
    {gc_bif,'-',{f,0},1,[{tr,{y,1},{t_integer,any}},{integer,1}],{x,1}}.
    {test_heap,2,2}.
    {put_list,{x,0},{y,0},{x,0}}.
    {swap,{x,0},{x,1}}.
    {call_last,2,{f,74},2}. % collect/2
  {label,77}.
    {loop_rec_end,{f,76}}.
  {label,78}.
    {wait,{f,76}}.


{function, module_info, 0, 80}.
  {label,79}.
    {line,[]}.
    {func_info,{atom,revcomp},{atom,module_info},0}.
  {label,80}.
    {move,{atom,revcomp},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 82}.
  {label,81}.
    {line,[]}.
    {func_info,{atom,revcomp},{atom,module_info},1}.
  {label,82}.
    {move,{x,0},{x,1}}.
    {move,{atom,revcomp},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-collect/2-lc$^0/1-0-', 1, 84}.
  {label,83}.
    {line,[{location,"revcomp.erl",182}]}.
    {func_info,{atom,revcomp},{atom,'-collect/2-lc$^0/1-0-'},1}.
  {label,84}.
    {test,is_nonempty_list,{f,86},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,0}}.
    {test,is_tuple,{f,85},[{x,1}]}.
    {test,test_arity,{f,85},[{x,1},2]}.
    {allocate,1,2}.
    {move,{x,1},{y,0}}.
    {call,1,{f,84}}. % '-collect/2-lc$^0/1-0-'/1
    {'%',{var_info,{x,0},[{type,{t_list,any,nil}}]}}.
    {test_heap,2,1}.
    {get_tuple_element,{y,0},1,{x,1}}.
    {put_list,{x,1},{x,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,85}.
    {call_only,1,{f,84}}. % '-collect/2-lc$^0/1-0-'/1
  {label,86}.
    {test,is_nil,{f,87},[{x,0}]}.
    return.
  {label,87}.
    {test_heap,3,1}.
    {put_tuple2,{x,0},{list,[{atom,bad_generator},{x,0}]}}.
    {call_ext_only,1,{extfunc,erlang,error,1}}.


{function, '-new_job/2-fun-0-', 3, 89}.
  {label,88}.
    {line,[{location,"revcomp.erl",179}]}.
    {func_info,{atom,revcomp},{atom,'-new_job/2-fun-0-'},3}.
  {label,89}.
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',
        {var_info,
            {x,1},
            [{type,
                 {t_cons,
                     {t_union,none,none,
                         {t_integer,any},
                         none,none,
                         {t_bitstring,8,true}},
                     nil}}]}}.
    {'%',{var_info,{x,2},[{type,{t_fun,4,{revcomp_chunk,4},any}}]}}.
    {allocate,1,3}.
    {move,{x,0},{y,0}}.
    {move,{x,2},{x,0}}.
    {call_ext,2,{extfunc,erlang,apply,2}}.
    {test_heap,3,1}.
    {put_tuple2,{x,1},{list,[{y,0},{x,0}]}}.
    {move,{atom,collector},{x,0}}.
    send.
    {deallocate,1}.
    return.


{function, '-revcomp/1-lc$^0/1-0-', 1, 91}.
  {label,90}.
    {line,[{location,"revcomp.erl",135}]}.
    {func_info,{atom,revcomp},{atom,'-revcomp/1-lc$^0/1-0-'},1}.
  {label,91}.
    {'%',{var_info,{x,0},[{type,{t_list,any,nil}}]}}.
    {test,is_nonempty_list,{f,92},[{x,0}]}.
    {allocate,1,1}.
    {get_list,{x,0},{x,0},{y,0}}.
    {call,1,{f,53}}. % complement/1
    {'%',{var_info,{x,0},[{type,{t_integer,{65,89}}}]}}.
    {swap,{y,0},{x,0}}.
    {call,1,{f,91}}. % '-revcomp/1-lc$^0/1-0-'/1
    {'%',{var_info,{x,0},[{type,{t_list,{t_integer,{65,89}},nil}}]}}.
    {test_heap,2,1}.
    {put_list,{y,0},{x,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,92}.
    return.


{function, '-loop/1-fun-1-', 2, 94}.
  {label,93}.
    {line,[{location,"revcomp.erl",40}]}.
    {func_info,{atom,revcomp},{atom,'-loop/1-fun-1-'},2}.
  {label,94}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {'%',{var_info,{x,1},[{type,{t_bs_matchable,1}}]}}.
    {test_heap,2,2}.
    {put_list,{x,1},{literal,"\n"},{x,1}}.
    {swap,{x,0},{x,1}}.
    {call_only,2,{f,28}}. % flush/2


{function, '-loop/1-fun-0-', 1, 96}.
  {label,95}.
    {line,[{location,"revcomp.erl",36}]}.
    {func_info,{atom,revcomp},{atom,'-loop/1-fun-0-'},1}.
  {label,96}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {move,{x,0},{x,1}}.
    {move,{literal,<<>>},{x,0}}.
    {call_only,2,{f,28}}. % flush/2
