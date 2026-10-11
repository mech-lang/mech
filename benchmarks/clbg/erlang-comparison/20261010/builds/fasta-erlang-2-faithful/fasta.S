{module, fasta}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 35}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"fasta.erl",20}]}.
    {func_info,{atom,fasta},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,1},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,1},[{x,2}]}.
    {allocate,2,2}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,1},{y,1}}.
    {move,{literal,[out,binary]},{x,1}}.
    {move,{literal,{fd,0,1}},{x,0}}.
    {line,[{location,"fasta.erl",21}]}.
    {call_ext,2,{extfunc,erlang,open_port,2}}.
    {move,{x,0},{x,1}}.
    {move,{atom,port},{x,0}}.
    {call_ext,2,{extfunc,erlang,put,2}}.
    {move,{y,1},{x,0}}.
    {init_yregs,{list,[{y,1}]}}.
    {line,[{location,"fasta.erl",24}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {move,{x,0},{y,1}}.
    {move,{literal,<<">ONE Homo sapiens alu\n">>},{x,0}}.
    {line,[{location,"fasta.erl",26}]}.
    {call,1,{f,28}}. % print/1
    {line,[{location,"fasta.erl",27}]}.
    {gc_bif,'*',{f,0},0,[{tr,{y,1},{t_integer,any}},{integer,2}],{x,1}}.
    {move,nil,{x,2}}.
    {move,{integer,0},{x,3}}.
    {move,{literal,<<"GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA">>},
          {x,0}}.
    {call,4,{f,4}}. % cycle/4
    {move,{literal,<<">TWO IUB ambiguity codes\n">>},{x,0}}.
    {line,[{location,"fasta.erl",29}]}.
    {call,1,{f,28}}. % print/1
    {move,{literal,[{97,0.27},
                    {99,0.12},
                    {103,0.12},
                    {116,0.27},
                    {66,0.02},
                    {68,0.02},
                    {72,0.02},
                    {75,0.02},
                    {77,0.02},
                    {78,0.02},
                    {82,0.02},
                    {83,0.02},
                    {86,0.02},
                    {87,0.02},
                    {89,0.02}]},
          {x,0}}.
    {line,[{location,"fasta.erl",30}]}.
    {call,1,{f,23}}. % mk_list/1
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {gc_bif,'*',{f,0},1,[{tr,{y,1},{t_integer,any}},{integer,3}],{x,2}}.
    {move,{integer,42},{x,4}}.
    {move,nil,{x,5}}.
    {move,nil,{x,3}}.
    {move,{integer,0},{x,6}}.
    {move,{integer,60},{x,1}}.
    {call,7,{f,12}}. % rand/7
    {'%',{var_info,{x,0},[{type,{t_integer,{0,139967}}}]}}.
    {move,{x,0},{y,0}}.
    {move,{literal,<<">THREE Homo sapiens frequency\n">>},{x,0}}.
    {line,[{location,"fasta.erl",32}]}.
    {call,1,{f,28}}. % print/1
    {move,{literal,[{97,0.302954942668},
                    {99,0.1979883004921},
                    {103,0.1975473066391},
                    {116,0.3015094502008}]},
          {x,0}}.
    {line,[{location,"fasta.erl",33}]}.
    {call,1,{f,23}}. % mk_list/1
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {gc_bif,'*',{f,0},1,[{tr,{y,1},{t_integer,any}},{integer,5}],{x,2}}.
    {move,{y,0},{x,4}}.
    {move,nil,{x,5}}.
    {move,nil,{x,3}}.
    {move,{integer,0},{x,6}}.
    {move,{integer,60},{x,1}}.
    {trim,2,0}.
    {call,7,{f,12}}. % rand/7
    {'%',{var_info,{x,0},[{type,{t_integer,{0,139967}}}]}}.
    {move,{integer,0},{x,0}}.
    {line,[{location,"fasta.erl",34}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.


{function, cycle, 4, 4}.
  {label,3}.
    {line,[{location,"fasta.erl",38}]}.
    {func_info,{atom,fasta},{atom,cycle},4}.
  {label,4}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_list,{t_bitstring,8,false},nil}}]}}.
    {'%',{var_info,{x,3},[{type,{t_integer,{0,'+inf'}}}]}}.
    {test,is_ge,{f,5},[{integer,59},{tr,{x,1},{t_integer,any}}]}.
    {line,[{location,"fasta.erl",39}]}.
    {bs_create_bin,{f,0},
                   0,3,8,
                   {x,0},
                   {list,[{atom,append},
                          1,8,nil,
                          {tr,{x,0},{t_bitstring,8,false}},
                          {atom,all},
                          {atom,string},
                          0,8,nil,
                          {string,<<"GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA">>},
                          {integer,287}]}}.
    {bs_start_match4,{atom,no_fail},3,{x,0},{x,3}}.
    {test,bs_get_binary2,
          {f,8},
          4,
          [{tr,{x,3},{t_bs_context,8}},
           {tr,{x,1},{t_integer,{'-inf',59}}},
           8,
           {field_flags,[{anno,[39,{file,"fasta.erl"}]},unsigned,big]}],
          {x,1}}.
    {allocate,1,3}.
    {move,{x,1},{y,0}}.
    {move,{x,2},{x,0}}.
    {line,[{location,"fasta.erl",40}]}.
    {call,1,{f,30}}. % reverse_print/1
    {line,[{location,"fasta.erl",41}]}.
    {bs_create_bin,{f,0},
                   0,0,8,
                   {x,0},
                   {list,[{atom,append},
                          1,8,nil,
                          {tr,{y,0},{t_bitstring,8,false}},
                          {atom,all},
                          {atom,string},
                          0,8,nil,
                          {string,<<"\n">>},
                          {integer,1}]}}.
    {call_last,1,{f,28},1}. % print/1
  {label,5}.
    {test,is_eq_exact,{f,6},[{tr,{x,3},{t_integer,{0,'+inf'}}},{integer,16}]}.
    {allocate,2,3}.
    {move,{x,1},{y,0}}.
    {move,{x,0},{y,1}}.
    {move,{x,2},{x,0}}.
    {line,[{location,"fasta.erl",44}]}.
    {call,1,{f,30}}. % reverse_print/1
    {move,nil,{x,2}}.
    {move,{y,0},{x,1}}.
    {move,{integer,0},{x,3}}.
    {move,{y,1},{x,0}}.
    {call_last,4,{f,4},2}. % cycle/4
  {label,6}.
    {line,[{location,"fasta.erl",47}]}.
    {gc_bif,byte_size,{f,0},4,[{tr,{x,0},{t_bitstring,8,false}}],{x,4}}.
    {test,is_ge,
          {f,7},
          [{integer,59},{tr,{x,4},{t_integer,{0,288230376151711743}}}]}.
    {line,[{location,"fasta.erl",48}]}.
    {bs_create_bin,{f,0},
                   0,4,8,
                   {x,0},
                   {list,[{atom,append},
                          1,8,nil,
                          {tr,{x,0},{t_bitstring,8,false}},
                          {atom,all},
                          {atom,string},
                          0,8,nil,
                          {string,<<"GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA">>},
                          {integer,287}]}}.
    {bs_start_match4,{atom,no_fail},4,{x,0},{x,4}}.
    {bs_match,{f,9},
              {x,4},
              {commands,[{ensure_at_least,480,1},
                         {binary,5,{literal,[]},60,8,{x,0}},
                         {get_tail,5,1,{x,4}}]}}.
    {line,[{location,"fasta.erl",49}]}.
    {gc_bif,'-',
            {f,0},
            5,
            [{tr,{x,1},{t_integer,{60,'+inf'}}},{integer,60}],
            {x,1}}.
    {bs_create_bin,{f,0},
                   0,5,8,
                   {x,0},
                   {list,[{atom,append},
                          1,8,nil,
                          {tr,{x,0},{t_bitstring,8,false}},
                          {atom,all},
                          {atom,string},
                          0,8,nil,
                          {string,<<"\n">>},
                          {integer,1}]}}.
    {gc_bif,'+',{f,0},5,[{tr,{x,3},{t_integer,{0,'+inf'}}},{integer,1}],{x,3}}.
    {test_heap,2,5}.
    {put_list,{x,0},{x,2},{x,2}}.
    {move,{x,4},{x,0}}.
    {call_only,4,{f,4}}. % cycle/4
  {label,7}.
    {bs_start_match4,{atom,no_fail},4,{x,0},{x,4}}.
    {bs_match,{f,10},
              {x,4},
              {commands,[{ensure_at_least,480,1},
                         {binary,5,{literal,[]},60,8,{x,0}},
                         {get_tail,5,1,{x,4}}]}}.
    {line,[{location,"fasta.erl",53}]}.
    {gc_bif,'-',
            {f,0},
            5,
            [{tr,{x,1},{t_integer,{60,'+inf'}}},{integer,60}],
            {x,1}}.
    {bs_create_bin,{f,0},
                   0,5,8,
                   {x,0},
                   {list,[{atom,append},
                          1,8,nil,
                          {tr,{x,0},{t_bitstring,8,false}},
                          {atom,all},
                          {atom,string},
                          0,8,nil,
                          {string,<<"\n">>},
                          {integer,1}]}}.
    {gc_bif,'+',{f,0},5,[{tr,{x,3},{t_integer,{0,'+inf'}}},{integer,1}],{x,3}}.
    {test_heap,2,5}.
    {put_list,{x,0},{x,2},{x,2}}.
    {move,{x,4},{x,0}}.
    {call_only,4,{f,4}}. % cycle/4
  {label,8}.
    {line,[{location,"fasta.erl",39}]}.
    {badmatch,{x,0}}.
  {label,9}.
    {line,[{location,"fasta.erl",48}]}.
    {badmatch,{x,0}}.
  {label,10}.
    {line,[{location,"fasta.erl",52}]}.
    {badmatch,{x,0}}.


{function, rand, 7, 12}.
  {label,11}.
    {line,[{location,"fasta.erl",55}]}.
    {func_info,{atom,fasta},{atom,rand},7}.
  {label,12}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,3},[{type,{t_list,{t_integer,{66,116}},nil}}]}}.
    {'%',{var_info,{x,4},[{type,{t_integer,{0,139967}}}]}}.
    {'%',
        {var_info,
            {x,5},
            [{type,
                 {t_list,
                     {t_cons,
                         {t_union,none,
                             {t_cons,{t_integer,{10,10}},nil},
                             {t_integer,{66,116}},
                             none,none,none},
                         nil},
                     nil}}]}}.
    {'%',{var_info,{x,6},[{type,{t_integer,{0,'+inf'}}}]}}.
    {test,is_eq_exact,{f,13},[{tr,{x,2},{t_integer,any}},{integer,0}]}.
    {allocate_heap,2,2,6}.
    {move,{x,5},{y,0}}.
    {move,{x,4},{y,1}}.
    {put_list,{literal,"\n"},{x,3},{x,0}}.
    {line,[{location,"fasta.erl",56}]}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {test_heap,2,1}.
    {put_list,{x,0},{y,0},{x,0}}.
    {trim,1,1}.
    {line,[{location,"fasta.erl",57}]}.
    {call,1,{f,30}}. % reverse_print/1
    {move,{y,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,13}.
    {test,is_eq_exact,{f,15},[{tr,{x,1},{t_integer,any}},{integer,0}]}.
    {test,is_eq_exact,{f,14},[{tr,{x,6},{t_integer,{0,'+inf'}}},{integer,16}]}.
    {allocate_heap,4,2,6}.
    {move,{x,5},{y,0}}.
    {move,{x,4},{y,1}}.
    {move,{x,2},{y,2}}.
    {move,{x,0},{y,3}}.
    {put_list,{literal,"\n"},{x,3},{x,0}}.
    {line,[{location,"fasta.erl",61}]}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {test_heap,2,1}.
    {put_list,{x,0},{y,0},{x,0}}.
    {trim,1,3}.
    {line,[{location,"fasta.erl",62}]}.
    {call,1,{f,30}}. % reverse_print/1
    {move,nil,{x,3}}.
    {move,{y,0},{x,4}}.
    {move,{y,1},{x,2}}.
    {move,nil,{x,5}}.
    {move,{integer,60},{x,1}}.
    {move,{integer,0},{x,6}}.
    {move,{y,2},{x,0}}.
    {call_last,7,{f,12},3}. % rand/7
  {label,14}.
    {allocate_heap,5,2,7}.
    {move,{x,6},{y,0}}.
    {move,{x,5},{y,1}}.
    {move,{x,4},{y,2}}.
    {move,{x,2},{y,3}}.
    {move,{x,0},{y,4}}.
    {put_list,{literal,"\n"},{x,3},{x,0}}.
    {line,[{location,"fasta.erl",66}]}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {line,[{location,"fasta.erl",67}]}.
    {gc_bif,'+',{f,0},1,[{tr,{y,0},{t_integer,{0,'+inf'}}},{integer,1}],{x,1}}.
    {test_heap,2,2}.
    {put_list,{x,0},{y,1},{x,5}}.
    {move,nil,{x,3}}.
    {move,{y,2},{x,4}}.
    {move,{y,3},{x,2}}.
    {move,{x,1},{x,6}}.
    {move,{integer,60},{x,1}}.
    {move,{y,4},{x,0}}.
    {call_last,7,{f,12},5}. % rand/7
  {label,15}.
    {allocate,7,7}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,6},{y,1}}.
    {move,{x,5},{y,2}}.
    {move,{x,3},{y,3}}.
    {move,{x,2},{y,4}}.
    {move,{x,1},{y,5}}.
    {move,{x,0},{y,6}}.
    {move,{x,4},{x,0}}.
    {line,[{location,"fasta.erl",70}]}.
    {call,1,{f,17}}. % random/1
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,2,true,
                                   #{1 => {t_float,any},
                                     2 => {t_integer,{0,139967}}}}}]}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{y,0}}.
    {move,{y,6},{x,0}}.
    {line,[{location,"fasta.erl",71}]}.
    {call,2,{f,19}}. % get_base/2
    {'%',{var_info,{x,0},[{type,{t_integer,{66,116}}}]}}.
    {line,[{location,"fasta.erl",72}]}.
    {gc_bif,'-',{f,0},1,[{tr,{y,5},{t_integer,any}},{integer,1}],{x,1}}.
    {gc_bif,'-',{f,0},2,[{tr,{y,4},{t_integer,any}},{integer,1}],{x,2}}.
    {test_heap,2,3}.
    {put_list,{x,0},{y,3},{x,3}}.
    {move,{y,0},{x,4}}.
    {move,{y,1},{x,6}}.
    {move,{y,2},{x,5}}.
    {move,{y,6},{x,0}}.
    {call_last,7,{f,12},7}. % rand/7


{function, random, 1, 17}.
  {label,16}.
    {line,[{location,"fasta.erl",74}]}.
    {func_info,{atom,fasta},{atom,random},1}.
  {label,17}.
    {'%',{var_info,{x,0},[{type,{t_integer,{0,139967}}}]}}.
    {line,[{location,"fasta.erl",75}]}.
    {gc_bif,'*',
            {f,0},
            1,
            [{tr,{x,0},{t_integer,{0,139967}}},{integer,3877}],
            {x,0}}.
    {gc_bif,'+',
            {f,0},
            1,
            [{tr,{x,0},{t_integer,{0,542652059}}},{integer,29573}],
            {x,0}}.
    {gc_bif,'rem',
            {f,0},
            1,
            [{tr,{x,0},{t_integer,{29573,542681632}}},{integer,139968}],
            {x,0}}.
    {line,[{location,"fasta.erl",76}]}.
    {fconv,{tr,{x,0},{t_integer,{0,139967}}},{fr,0}}.
    {fmove,{float,139968.0},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,3},{floats,1},{funs,0}]},1}.
    {fmove,{fr,0},{x,1}}.
    {put_tuple2,{x,0},{list,[{x,1},{x,0}]}}.
    return.


{function, get_base, 2, 19}.
  {label,18}.
    {line,[{location,"fasta.erl",78}]}.
    {func_info,{atom,fasta},{atom,get_base},2}.
  {label,19}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,any}}]}}.
    {get_list,{x,0},{x,2},{x,0}}.
    {get_tuple_element,{x,2},0,{x,3}}.
    {test,is_ne_exact,
          {f,20},
          [{tr,{x,0},
               {t_list,{t_tuple,2,true,
                                #{1 => {t_integer,{66,116}},
                                  2 => {t_float,any}}},
                       nil}},
           nil]}.
    {get_tuple_element,{x,2},1,{x,2}}.
    {test,is_lt,{f,21},[{tr,{x,1},{t_float,any}},{tr,{x,2},{t_float,any}}]}.
  {label,20}.
    {move,{x,3},{x,0}}.
    return.
  {label,21}.
    {call_only,2,{f,19}}. % get_base/2


{function, mk_list, 1, 23}.
  {label,22}.
    {line,[{location,"fasta.erl",83}]}.
    {func_info,{atom,fasta},{atom,mk_list},1}.
  {label,23}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},
                           2 => {t_float,{0.02,0.302954942668}}}},
                     nil}}]}}.
    {allocate,0,1}.
    {move,nil,{x,2}}.
    {move,{integer,0},{x,1}}.
    {call,3,{f,25}}. % mk_list/3
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {call_ext_last,1,{extfunc,lists,reverse,1},0}.


{function, mk_list, 3, 25}.
  {label,24}.
    {line,[{location,"fasta.erl",84}]}.
    {func_info,{atom,fasta},{atom,mk_list},3}.
  {label,25}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},
                           2 => {t_float,{0.02,0.302954942668}}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_number,any}}]}}.
    {'%',
        {var_info,
            {x,2},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {get_list,{x,0},{x,3},{x,0}}.
    {get_tuple_element,{x,3},0,{x,4}}.
    {get_tuple_element,{x,3},1,{x,3}}.
    {test,is_nil,
          {f,26},
          [{tr,{x,0},
               {t_list,{t_tuple,2,true,
                                #{1 => {t_integer,{66,116}},
                                  2 => {t_float,{0.02,0.302954942668}}}},
                       nil}}]}.
    {fconv,{tr,{x,1},{t_number,any}},{fr,0}}.
    {fmove,{x,3},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,5},{floats,1},{funs,0}]},5}.
    {fmove,{fr,0},{x,0}}.
    {put_tuple2,{x,0},{list,[{x,4},{x,0}]}}.
    {put_list,{x,0},{x,2},{x,0}}.
    return.
  {label,26}.
    {line,[{location,"fasta.erl",86}]}.
    {fconv,{tr,{x,1},{t_number,any}},{fr,0}}.
    {fmove,{x,3},{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {test_heap,{alloc,[{words,5},{floats,1},{funs,0}]},5}.
    {fmove,{fr,0},{x,1}}.
    {put_tuple2,{x,3},{list,[{x,4},{x,1}]}}.
    {put_list,{x,3},{x,2},{x,2}}.
    {call_only,3,{f,25}}. % mk_list/3


{function, print, 1, 28}.
  {label,27}.
    {line,[{location,"fasta.erl",88}]}.
    {func_info,{atom,fasta},{atom,print},1}.
  {label,28}.
    {'%',{var_info,{x,0},[{type,{t_bitstring,8,false}}]}}.
    {move,{x,0},{x,1}}.
    {bif,get,{f,0},[{atom,port}],{x,0}}.
    {call_ext_only,2,{extfunc,erlang,port_command,2}}.


{function, reverse_print, 1, 30}.
  {label,29}.
    {line,[{location,"fasta.erl",89}]}.
    {func_info,{atom,fasta},{atom,reverse_print},1}.
  {label,30}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_union,none,
                         {t_cons,
                             {t_union,none,
                                 {t_cons,{t_integer,{10,10}},nil},
                                 {t_integer,{66,116}},
                                 none,none,none},
                             nil},
                         none,none,none,
                         {t_bitstring,8,false}},
                     nil}}]}}.
    {allocate,1,1}.
    {bif,get,{f,0},[{atom,port}],{y,0}}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {move,{x,0},{x,1}}.
    {move,{y,0},{x,0}}.
    {call_ext_last,2,{extfunc,erlang,port_command,2},1}.


{function, module_info, 0, 32}.
  {label,31}.
    {line,[]}.
    {func_info,{atom,fasta},{atom,module_info},0}.
  {label,32}.
    {move,{atom,fasta},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 34}.
  {label,33}.
    {line,[]}.
    {func_info,{atom,fasta},{atom,module_info},1}.
  {label,34}.
    {move,{x,0},{x,1}}.
    {move,{atom,fasta},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.
