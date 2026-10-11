{module, revcomp}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 90}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"revcomp.erl",26}]}.
    {func_info,{atom,revcomp},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,1},[{x,0}]}.
    {get_tl,{x,0},{x,1}}.
    {test,is_nil,{f,1},[{x,1}]}.
    {allocate,0,0}.
    {move,{literal,[binary]},{x,0}}.
    {line,[{location,"revcomp.erl",27}]}.
    {call_ext,1,{extfunc,io,setopts,1}}.
    {line,[{location,"revcomp.erl",28}]}.
    {call,0,{f,4}}. % run_parallel/0
    {'%',{var_info,{x,0},[{type,{t_atom,[ok]}}]}}.
    {line,[{location,"revcomp.erl",29}]}.
    {call_ext_last,0,{extfunc,erlang,halt,0},0}.


{function, run_parallel, 0, 4}.
  {label,3}.
    {line,[{location,"revcomp.erl",33}]}.
    {func_info,{atom,revcomp},{atom,run_parallel},0}.
  {label,4}.
    {allocate,0,0}.
    {bif,self,{f,0},[],{x,1}}.
    {move,{atom,reader},{x,0}}.
    {line,[{location,"revcomp.erl",34}]}.
    {call_ext,2,{extfunc,erlang,register,2}}.
    {move,{atom,go},{x,1}}.
    {move,{atom,reader},{x,0}}.
    {line,[{location,"revcomp.erl",35}]}.
    send.
    {move,{integer,256},{x,0}}.
    bs_init_writable.
    {call_last,1,{f,6},0}. % loop/1


{function, loop, 1, 6}.
  {label,5}.
    {line,[{location,"revcomp.erl",38}]}.
    {func_info,{atom,revcomp},{atom,loop},1}.
  {label,6}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {allocate,2,1}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,0},{y,1}}.
    {move,{atom,''},{x,0}}.
    {line,[{location,"revcomp.erl",39}]}.
    {call_ext,1,{extfunc,io,get_line,1}}.
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
    {make_fun3,{f,85},0,0,{x,0},{list,[{y,1},{y,0}]}}.
    {init_yregs,{list,[{y,0},{y,1}]}}.
    {line,[{location,"revcomp.erl",46}]}.
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
    {make_fun3,{f,87},0,0,{x,0},{list,[{y,1}]}}.
    {init_yregs,{list,[{y,0},{y,1}]}}.
    {line,[{location,"revcomp.erl",42}]}.
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
    {line,[{location,"revcomp.erl",50}]}.
    {gc_bif,size,{f,0},0,[{y,0}],{x,0}}.
    {gc_bif,'-',{f,0},1,[{tr,{x,0},{t_integer,any}},{integer,1}],{x,0}}.
    {test,bs_start_match3,{f,18},1,[{y,0}],{x,1}}.
    {test,bs_get_binary2,
          {f,18},
          2,
          [{tr,{x,1},{t_bs_context,1}},
           {tr,{x,0},{t_integer,any}},
           8,
           {field_flags,[{anno,[51,{file,"revcomp.erl"}]},unsigned,big]}],
          {x,0}}.
    {bs_match,{f,18},{x,1},{commands,[{ensure_exactly,8}]}}.
    {line,[{location,"revcomp.erl",52}]}.
    {bs_create_bin,{f,0},
                   0,1,8,
                   {x,0},
                   {list,[{atom,private_append},
                          1,8,nil,
                          {tr,{y,1},{t_bitstring,8,true}},
                          {atom,all},
                          {atom,binary},
                          2,8,nil,
                          {tr,{x,0},{t_bitstring,8,false}},
                          {atom,all}]}}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {call_last,1,{f,6},2}. % loop/1
  {label,18}.
    {line,[{location,"revcomp.erl",51}]}.
    {badmatch,{y,0}}.


{function, flush, 2, 20}.
  {label,19}.
    {line,[{location,"revcomp.erl",58}]}.
    {func_info,{atom,revcomp},{atom,flush},2}.
  {label,20}.
    {'%',{var_info,{x,0},[{type,{t_bs_matchable,1}}]}}.
    {'%',{var_info,{x,1},[{type,{t_bitstring,8,true}}]}}.
    {allocate,2,2}.
    {move,{x,1},{y,0}}.
    {move,{x,0},{y,1}}.
    {bif,self,{f,0},[],{x,1}}.
    {move,{atom,collector},{x,0}}.
    {line,[{location,"revcomp.erl",59}]}.
    {call_ext,2,{extfunc,erlang,register,2}}.
    {move,{y,0},{x,0}}.
    {trim,1,1}.
    {line,[{location,"revcomp.erl",60}]}.
    {call,1,{f,22}}. % reverse_complement/1
    {'%',{var_info,{x,0},
                   [{type,{t_union,none,
                                   {t_list,any,nil},
                                   none,none,none,
                                   {t_bitstring,256,true}}}]}}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {move,{y,0},{x,0}}.
    {trim,1,0}.
    {line,[{location,"revcomp.erl",61}]}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {move,{atom,collector},{x,0}}.
    {line,[{location,"revcomp.erl",62}]}.
    {call_ext,1,{extfunc,erlang,unregister,1}}.
    {move,{atom,go},{x,1}}.
    {move,{atom,reader},{x,0}}.
    {line,[{location,"revcomp.erl",63}]}.
    send.
    {deallocate,0}.
    return.


{function, reverse_complement, 1, 22}.
  {label,21}.
    {line,[{location,"revcomp.erl",69}]}.
    {func_info,{atom,revcomp},{atom,reverse_complement},1}.
  {label,22}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {test,is_eq_exact,{f,23},[{tr,{x,0},{t_bitstring,8,true}},{literal,<<>>}]}.
    return.
  {label,23}.
    {line,[{location,"revcomp.erl",72}]}.
    {gc_bif,byte_size,{f,0},1,[{tr,{x,0},{t_bitstring,8,true}}],{x,1}}.
    {line,[{location,"revcomp.erl",117}]}.
    {gc_bif,'div',
            {f,0},
            2,
            [{tr,{x,1},{t_integer,{0,288230376151711743}}},{integer,4}],
            {x,2}}.
    {line,[{location,"revcomp.erl",118}]}.
    {gc_bif,'rem',
            {f,0},
            3,
            [{tr,{x,2},{t_integer,{0,72057594037927935}}},{integer,60}],
            {x,3}}.
    {line,[{location,"revcomp.erl",119}]}.
    {gc_bif,'-',
            {f,0},
            4,
            [{tr,{x,2},{t_integer,{0,72057594037927935}}},
             {tr,{x,3},{t_integer,{0,59}}}],
            {x,2}}.
    {line,[{location,"revcomp.erl",120}]}.
    {gc_bif,'rem',
            {f,0},
            4,
            [{tr,{x,1},{t_integer,{0,288230376151711743}}},{integer,4}],
            {x,1}}.
    {gc_bif,'*',{f,0},4,[{tr,{x,3},{t_integer,{0,59}}},{integer,4}],{x,3}}.
    {gc_bif,'+',
            {f,0},
            4,
            [{tr,{x,1},{t_integer,{0,3}}},{tr,{x,3},{t_integer,{0,236}}}],
            {x,1}}.
    {allocate,3,3}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,1},{y,1}}.
    {move,{x,0},{y,2}}.
    {move,{x,2},{x,1}}.
    {line,[{location,"revcomp.erl",73}]}.
    {call,2,{f,25}}. % start_jobs/2
    {'%',{var_info,{x,0},[{type,{t_integer,{0,4}}}]}}.
    {move,{x,0},{y,0}}.
    {move,{integer,256},{x,0}}.
    bs_init_writable.
    {move,{x,0},{x,2}}.
    {move,{y,1},{x,1}}.
    {move,{y,2},{x,0}}.
    {init_yregs,{list,[{y,1},{y,2}]}}.
    {line,[{location,"revcomp.erl",74}]}.
    {call,3,{f,35}}. % revcomp_last/3
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {move,{x,0},{y,2}}.
    {move,{y,0},{x,0}}.
    {trim,2,1}.
    {line,[{location,"revcomp.erl",75}]}.
    {call,1,{f,43}}. % collect/1
    {'%',{var_info,{x,0},[{type,{t_list,any,nil}}]}}.
    {test_heap,2,1}.
    {put_list,{y,0},nil,{x,1}}.
    {call_ext_last,2,{extfunc,erlang,'++',2},1}.


{function, start_jobs, 2, 25}.
  {label,24}.
    {line,[{location,"revcomp.erl",77}]}.
    {func_info,{atom,revcomp},{atom,start_jobs},2}.
  {label,25}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {test,is_eq_exact,{f,26},[{tr,{x,1},{t_integer,any}},{integer,0}]}.
    {move,{integer,0},{x,0}}.
    return.
  {label,26}.
    {line,[{location,"revcomp.erl",80}]}.
    {gc_bif,byte_size,{f,0},2,[{tr,{x,0},{t_bitstring,8,true}}],{x,2}}.
    {move,{integer,0},{x,3}}.
    {call_only,4,{f,28}}. % start_jobs/4


{function, start_jobs, 4, 28}.
  {label,27}.
    {line,[{location,"revcomp.erl",82}]}.
    {func_info,{atom,revcomp},{atom,start_jobs},4}.
  {label,28}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,3},[{type,{t_integer,{0,'+inf'}}}]}}.
    {test,is_eq_exact,{f,29},[{tr,{x,3},{t_integer,{0,'+inf'}}},{integer,4}]}.
    {move,{x,3},{x,0}}.
    return.
  {label,29}.
    {test,is_ge,
          {f,27},
          [{tr,{x,2},{t_integer,any}},{tr,{x,1},{t_integer,any}}]}.
    {test_heap,{alloc,[{words,0},{floats,0},{funs,1}]},4}.
    {make_fun3,{f,31},0,0,{x,4},{list,[]}}.
    {line,[{location,"revcomp.erl",85}]}.
    {gc_bif,'-',
            {f,0},
            5,
            [{tr,{x,2},{t_integer,any}},{tr,{x,1},{t_integer,any}}],
            {x,5}}.
    {allocate_heap,4,{alloc,[{words,9},{floats,0},{funs,1}]},6}.
    {move,{x,3},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,1},{y,2}}.
    {move,{x,0},{y,3}}.
    {put_list,{x,2},{literal,[<<>>]},{x,0}}.
    {put_list,{x,5},{x,0},{x,0}}.
    {put_list,{y,3},{x,0},{x,0}}.
    {make_fun3,{f,83},0,0,{x,0},{list,[{x,3},{x,4},{x,0}]}}.
    {line,[{location,"revcomp.erl",158}]}.
    {call_ext,1,{extfunc,erlang,spawn,1}}.
    {line,[{location,"revcomp.erl",86}]}.
    {gc_bif,'-',
            {f,0},
            0,
            [{tr,{y,1},{t_integer,any}},{tr,{y,2},{t_integer,any}}],
            {x,0}}.
    {gc_bif,'+',{f,0},1,[{tr,{y,0},{t_integer,{0,'+inf'}}},{integer,1}],{x,3}}.
    {move,{y,2},{x,1}}.
    {move,{x,0},{x,2}}.
    {move,{y,3},{x,0}}.
    {call_last,4,{f,28},4}. % start_jobs/4


{function, revcomp_chunk, 4, 31}.
  {label,30}.
    {line,[{location,"revcomp.erl",89}]}.
    {func_info,{atom,revcomp},{atom,revcomp_chunk},4}.
  {label,31}.
    {test,is_eq_exact,{f,32},[{x,2},{x,1}]}.
    {move,{x,3},{x,0}}.
    return.
  {label,32}.
    {line,[{location,"revcomp.erl",92}]}.
    {gc_bif,'-',{f,0},4,[{x,2},{integer,60}],{x,2}}.
    {test,bs_start_match3,{f,33},4,[{x,0}],{x,4}}.
    {test,bs_skip_bits2,
          {f,33},
          [{x,4},
           {tr,{x,2},{t_number,any}},
           8,
           {field_flags,[{anno,[93,{file,"revcomp.erl"}]},unsigned,big]}]}.
    {bs_match,{f,33},
              {x,4},
              {commands,[{ensure_at_least,480,8},
                         {binary,5,{literal,[]},60,8,{x,5}}]}}.
    {allocate,4,6}.
    {move,{x,2},{y,0}}.
    {move,{x,3},{y,1}}.
    {move,{x,1},{y,2}}.
    {move,{x,0},{y,3}}.
    {move,{x,5},{x,0}}.
    {line,[{location,"revcomp.erl",94}]}.
    {call,1,{f,40}}. % revcomp/1
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {line,[{location,"revcomp.erl",95}]}.
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
    {call_last,4,{f,31},4}. % revcomp_chunk/4
  {label,33}.
    {line,[{location,"revcomp.erl",93}]}.
    {badmatch,{x,0}}.


{function, revcomp_last, 3, 35}.
  {label,34}.
    {line,[{location,"revcomp.erl",98}]}.
    {func_info,{atom,revcomp},{atom,revcomp_last},3}.
  {label,35}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_bitstring,8,true}}]}}.
    {test,is_ge,{f,36},[{tr,{x,1},{t_integer,any}},{integer,61}]}.
    {line,[{location,"revcomp.erl",99}]}.
    {gc_bif,'-',
            {f,0},
            3,
            [{tr,{x,1},{t_integer,{61,'+inf'}}},{integer,60}],
            {x,1}}.
    {bs_start_match4,{atom,no_fail},3,{x,0},{x,3}}.
    {test,bs_skip_bits2,
          {f,37},
          [{x,3},
           {tr,{x,1},{t_integer,{1,'+inf'}}},
           8,
           {field_flags,[{anno,[100,{file,"revcomp.erl"}]},unsigned,big]}]}.
    {bs_match,{f,37},
              {x,3},
              {commands,[{ensure_at_least,480,1},
                         {binary,4,{literal,[]},60,8,{x,3}}]}}.
    {allocate,3,4}.
    {move,{x,1},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,0},{y,2}}.
    {move,{x,3},{x,0}}.
    {line,[{location,"revcomp.erl",101}]}.
    {call,1,{f,40}}. % revcomp/1
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {line,[{location,"revcomp.erl",102}]}.
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
    {call_last,3,{f,35},3}. % revcomp_last/3
  {label,36}.
    {bs_start_match4,{atom,no_fail},3,{x,0},{x,3}}.
    {test,bs_get_binary2,
          {f,38},
          4,
          [{tr,{x,3},{t_bs_context,8}},
           {tr,{x,1},{t_integer,{'-inf',60}}},
           8,
           {field_flags,[{anno,[104,{file,"revcomp.erl"}]},unsigned,big]}],
          {x,1}}.
    {allocate,1,3}.
    {move,{x,2},{y,0}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"revcomp.erl",105}]}.
    {call,1,{f,40}}. % revcomp/1
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {line,[{location,"revcomp.erl",106}]}.
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
  {label,37}.
    {line,[{location,"revcomp.erl",100}]}.
    {badmatch,{x,0}}.
  {label,38}.
    {line,[{location,"revcomp.erl",104}]}.
    {badmatch,{x,0}}.


{function, revcomp, 1, 40}.
  {label,39}.
    {line,[{location,"revcomp.erl",110}]}.
    {func_info,{atom,revcomp},{atom,revcomp},1}.
  {label,40}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {test,is_eq_exact,
          {f,41},
          [{tr,{x,0},{t_bitstring,8,false}},{literal,<<>>}]}.
    return.
  {label,41}.
    {allocate,0,1}.
    {line,[{location,"revcomp.erl",114}]}.
    {call_ext,1,{extfunc,erlang,binary_to_list,1}}.
    {call,1,{f,63}}. % '-revcomp/1-lc$^0/1-0-'/1
    {'%',{var_info,{x,0},[{type,{t_list,{t_integer,{65,89}},nil}}]}}.
    {test_heap,2,1}.
    {put_list,{integer,10},{x,0},{x,0}}.
    {line,[{location,"revcomp.erl",113}]}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {call_ext_last,1,{extfunc,erlang,list_to_binary,1},0}.


{function, collect, 1, 43}.
  {label,42}.
    {line,[{location,"revcomp.erl",160}]}.
    {func_info,{atom,revcomp},{atom,collect},1}.
  {label,43}.
    {'%',{var_info,{x,0},[{type,{t_integer,{0,4}}}]}}.
    {move,nil,{x,1}}.
    {call_only,2,{f,45}}. % collect/2


{function, collect, 2, 45}.
  {label,44}.
    {line,[{location,"revcomp.erl",161}]}.
    {func_info,{atom,revcomp},{atom,collect},2}.
  {label,45}.
    {'%',{var_info,{x,0},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,1},[{type,{t_list,{t_tuple,2,true,#{}},nil}}]}}.
    {test,is_eq_exact,{f,49},[{tr,{x,0},{t_integer,any}},{integer,0}]}.
    {allocate,1,2}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{integer,1},{x,0}}.
    {call_ext,2,{extfunc,lists,keysort,2}}.
    {test,is_nonempty_list,{f,47},[{x,0}]}.
    {get_list,{x,0},{y,0},{x,0}}.
    {test,is_tuple,{f,46},[{y,0}]}.
    {test,test_arity,{f,46},[{y,0},2]}.
    {call,1,{f,58}}. % '-collect/2-lc$^0/1-0-'/1
    {'%',{var_info,{x,0},[{type,{t_list,any,nil}}]}}.
    {test_heap,2,1}.
    {get_tuple_element,{y,0},1,{x,1}}.
    {put_list,{x,1},{x,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,46}.
    {call_last,1,{f,58},1}. % '-collect/2-lc$^0/1-0-'/1
  {label,47}.
    {test,is_nil,{f,48},[{x,0}]}.
    {deallocate,1}.
    return.
  {label,48}.
    {test_heap,3,1}.
    {put_tuple2,{x,0},{list,[{atom,bad_generator},{x,0}]}}.
    {call_ext_last,1,{extfunc,erlang,error,1},1}.
  {label,49}.
    {allocate,2,2}.
    {move,{x,1},{y,0}}.
    {move,{x,0},{y,1}}.
  {label,50}.
    {loop_rec,{f,52},{x,0}}.
    {test,is_tuple,{f,51},[{x,0}]}.
    {test,test_arity,{f,51},[{x,0},2]}.
    remove_message.
    {line,[{location,"revcomp.erl",162}]}.
    {gc_bif,'-',{f,0},1,[{tr,{y,1},{t_integer,any}},{integer,1}],{x,1}}.
    {test_heap,2,2}.
    {put_list,{x,0},{y,0},{x,0}}.
    {swap,{x,0},{x,1}}.
    {call_last,2,{f,45},2}. % collect/2
  {label,51}.
    {loop_rec_end,{f,50}}.
  {label,52}.
    {wait,{f,50}}.


{function, module_info, 0, 54}.
  {label,53}.
    {line,[]}.
    {func_info,{atom,revcomp},{atom,module_info},0}.
  {label,54}.
    {move,{atom,revcomp},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 56}.
  {label,55}.
    {line,[]}.
    {func_info,{atom,revcomp},{atom,module_info},1}.
  {label,56}.
    {move,{x,0},{x,1}}.
    {move,{atom,revcomp},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-collect/2-lc$^0/1-0-', 1, 58}.
  {label,57}.
    {line,[{location,"revcomp.erl",161}]}.
    {func_info,{atom,revcomp},{atom,'-collect/2-lc$^0/1-0-'},1}.
  {label,58}.
    {test,is_nonempty_list,{f,60},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,0}}.
    {test,is_tuple,{f,59},[{x,1}]}.
    {test,test_arity,{f,59},[{x,1},2]}.
    {allocate,1,2}.
    {move,{x,1},{y,0}}.
    {call,1,{f,58}}. % '-collect/2-lc$^0/1-0-'/1
    {'%',{var_info,{x,0},[{type,{t_list,any,nil}}]}}.
    {test_heap,2,1}.
    {get_tuple_element,{y,0},1,{x,1}}.
    {put_list,{x,1},{x,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,59}.
    {call_only,1,{f,58}}. % '-collect/2-lc$^0/1-0-'/1
  {label,60}.
    {test,is_nil,{f,61},[{x,0}]}.
    return.
  {label,61}.
    {test_heap,3,1}.
    {put_tuple2,{x,0},{list,[{atom,bad_generator},{x,0}]}}.
    {call_ext_only,1,{extfunc,erlang,error,1}}.


{function, '-revcomp/1-lc$^0/1-0-', 1, 63}.
  {label,62}.
    {line,[{location,"revcomp.erl",114}]}.
    {func_info,{atom,revcomp},{atom,'-revcomp/1-lc$^0/1-0-'},1}.
  {label,63}.
    {'%',{var_info,{x,0},[{type,{t_list,any,nil}}]}}.
    {test,is_nonempty_list,{f,81},[{x,0}]}.
    {allocate,1,1}.
    {init_yregs,{list,[{y,0}]}}.
    {get_list,{x,0},{x,1},{x,0}}.
    {select_val,{x,1},
                {f,80},
                {list,[{integer,65},
                       {f,78},
                       {integer,66},
                       {f,77},
                       {integer,67},
                       {f,76},
                       {integer,68},
                       {f,75},
                       {integer,71},
                       {f,74},
                       {integer,72},
                       {f,73},
                       {integer,75},
                       {f,72},
                       {integer,77},
                       {f,71},
                       {integer,78},
                       {f,70},
                       {integer,82},
                       {f,69},
                       {integer,83},
                       {f,68},
                       {integer,84},
                       {f,67},
                       {integer,85},
                       {f,67},
                       {integer,86},
                       {f,66},
                       {integer,87},
                       {f,65},
                       {integer,89},
                       {f,64},
                       {integer,97},
                       {f,78},
                       {integer,98},
                       {f,77},
                       {integer,99},
                       {f,76},
                       {integer,100},
                       {f,75},
                       {integer,103},
                       {f,74},
                       {integer,104},
                       {f,73},
                       {integer,107},
                       {f,72},
                       {integer,109},
                       {f,71},
                       {integer,110},
                       {f,70},
                       {integer,114},
                       {f,69},
                       {integer,115},
                       {f,68},
                       {integer,116},
                       {f,67},
                       {integer,117},
                       {f,67},
                       {integer,118},
                       {f,66},
                       {integer,119},
                       {f,65},
                       {integer,121},
                       {f,64}]}}.
  {label,64}.
    {move,{integer,82},{y,0}}.
    {jump,{f,79}}.
  {label,65}.
    {move,{integer,87},{y,0}}.
    {jump,{f,79}}.
  {label,66}.
    {move,{integer,66},{y,0}}.
    {jump,{f,79}}.
  {label,67}.
    {move,{integer,65},{y,0}}.
    {jump,{f,79}}.
  {label,68}.
    {move,{integer,83},{y,0}}.
    {jump,{f,79}}.
  {label,69}.
    {move,{integer,89},{y,0}}.
    {jump,{f,79}}.
  {label,70}.
    {move,{integer,78},{y,0}}.
    {jump,{f,79}}.
  {label,71}.
    {move,{integer,75},{y,0}}.
    {jump,{f,79}}.
  {label,72}.
    {move,{integer,77},{y,0}}.
    {jump,{f,79}}.
  {label,73}.
    {move,{integer,68},{y,0}}.
    {jump,{f,79}}.
  {label,74}.
    {move,{integer,67},{y,0}}.
    {jump,{f,79}}.
  {label,75}.
    {move,{integer,72},{y,0}}.
    {jump,{f,79}}.
  {label,76}.
    {move,{integer,71},{y,0}}.
    {jump,{f,79}}.
  {label,77}.
    {move,{integer,86},{y,0}}.
    {jump,{f,79}}.
  {label,78}.
    {move,{integer,84},{y,0}}.
  {label,79}.
    {call,1,{f,63}}. % '-revcomp/1-lc$^0/1-0-'/1
    {'%',{var_info,{x,0},[{type,{t_list,{t_integer,{65,89}},nil}}]}}.
    {test_heap,2,1}.
    {put_list,{y,0},{x,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,80}.
    {move,{x,1},{x,0}}.
    {call_last,1,{f,89},1}. % '-inlined-complement/1-'/1
  {label,81}.
    return.


{function, '-start_jobs/4-anonymous-0-', 3, 83}.
  {label,82}.
    {line,[{location,"revcomp.erl",158}]}.
    {func_info,{atom,revcomp},{atom,'-start_jobs/4-anonymous-0-'},3}.
  {label,83}.
    {'%',{var_info,{x,0},[{type,{t_integer,{0,'+inf'}}}]}}.
    {'%',{var_info,{x,1},[{type,{t_fun,4,{revcomp_chunk,4},any}}]}}.
    {'%',
        {var_info,
            {x,2},
            [{type,
                 {t_cons,
                     {t_union,none,none,
                         {t_integer,any},
                         none,none,
                         {t_bitstring,8,true}},
                     nil}}]}}.
    {allocate,1,3}.
    {move,{x,0},{y,0}}.
    {move,{x,1},{x,0}}.
    {move,{x,2},{x,1}}.
    {call_ext,2,{extfunc,erlang,apply,2}}.
    {test_heap,3,1}.
    {put_tuple2,{x,1},{list,[{y,0},{x,0}]}}.
    {move,{atom,collector},{x,0}}.
    send.
    {deallocate,1}.
    return.


{function, '-loop/1-anonymous-1-', 2, 85}.
  {label,84}.
    {line,[{location,"revcomp.erl",46}]}.
    {func_info,{atom,revcomp},{atom,'-loop/1-anonymous-1-'},2}.
  {label,85}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {'%',{var_info,{x,1},[{type,{t_bs_matchable,1}}]}}.
    {swap,{x,0},{x,1}}.
    {call_only,2,{f,20}}. % flush/2


{function, '-loop/1-anonymous-0-', 1, 87}.
  {label,86}.
    {line,[{location,"revcomp.erl",42}]}.
    {func_info,{atom,revcomp},{atom,'-loop/1-anonymous-0-'},1}.
  {label,87}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,true}}]}}.
    {move,{x,0},{x,1}}.
    {move,{literal,<<>>},{x,0}}.
    {call_only,2,{f,20}}. % flush/2


{function, '-inlined-complement/1-', 1, 89}.
  {label,88}.
    {line,[{location,"revcomp.erl",123}]}.
    {func_info,{atom,revcomp},{atom,'-inlined-complement/1-'},1}.
  {label,89}.
    {jump,{f,88}}.
