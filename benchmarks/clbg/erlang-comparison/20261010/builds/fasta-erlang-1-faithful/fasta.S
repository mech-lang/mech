{module, fasta}.  %% version = 0

{exports, [{main,0},{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 40}.


{function, main, 0, 2}.
  {label,1}.
    {line,[{location,"fasta.erl",22}]}.
    {func_info,{atom,fasta},{atom,main},0}.
  {label,2}.
    {move,{literal,["1000"]},{x,0}}.
    {call_only,1,{f,4}}. % main/1


{function, main, 1, 4}.
  {label,3}.
    {line,[{location,"fasta.erl",23}]}.
    {func_info,{atom,fasta},{atom,main},1}.
  {label,4}.
    {test,is_nonempty_list,{f,3},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,3},[{x,2}]}.
    {allocate,2,2}.
    {init_yregs,{list,[{y,0},{y,1}]}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"fasta.erl",24}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {line,[{location,"fasta.erl",26}]}.
    {gc_bif,'*',{f,0},1,[{tr,{x,0},{t_integer,any}},{integer,2}],{x,3}}.
    {move,{x,0},{y,1}}.
    {move,{literal,"Homo sapiens alu"},{x,1}}.
    {move,{literal,"GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"},
          {x,2}}.
    {move,{literal,"ONE"},{x,0}}.
    {call,4,{f,6}}. % write_fasta_cycle/4
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
    {line,[{location,"fasta.erl",27}]}.
    {call,1,{f,32}}. % cumulative/1
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {gc_bif,'*',{f,0},1,[{tr,{y,1},{t_integer,any}},{integer,3}],{x,3}}.
    {move,{x,0},{x,2}}.
    {move,{literal,"IUB ambiguity codes"},{x,1}}.
    {move,{integer,42},{x,4}}.
    {move,{literal,"TWO"},{x,0}}.
    {call,5,{f,18}}. % write_fasta_rand/5
    {'%',{var_info,{x,0},[{type,{t_integer,{0,139967}}}]}}.
    {move,{x,0},{y,0}}.
    {move,{literal,[{97,0.302954942668},
                    {99,0.1979883004921},
                    {103,0.1975473066391},
                    {116,0.3015094502008}]},
          {x,0}}.
    {line,[{location,"fasta.erl",28}]}.
    {call,1,{f,32}}. % cumulative/1
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {gc_bif,'*',{f,0},1,[{tr,{y,1},{t_integer,any}},{integer,5}],{x,3}}.
    {move,{x,0},{x,2}}.
    {move,{literal,"Homo sapiens frequency"},{x,1}}.
    {move,{y,0},{x,4}}.
    {trim,2,0}.
    {move,{literal,"THREE"},{x,0}}.
    {call,5,{f,18}}. % write_fasta_rand/5
    {'%',{var_info,{x,0},[{type,{t_integer,{0,139967}}}]}}.
    {move,{integer,0},{x,0}}.
    {line,[{location,"fasta.erl",29}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.


{function, write_fasta_cycle, 4, 6}.
  {label,5}.
    {line,[{location,"fasta.erl",32}]}.
    {func_info,{atom,fasta},{atom,write_fasta_cycle},4}.
  {label,6}.
    {'%',{var_info,{x,0},[{type,{t_cons,{t_integer,{69,79}},nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_cons,{t_integer,{32,117}},nil}}]}}.
    {'%',{var_info,{x,2},[{type,{t_cons,{t_integer,{65,84}},nil}}]}}.
    {'%',{var_info,{x,3},[{type,{t_integer,any}}]}}.
    {allocate,3,4}.
    {move,{x,3},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,0},{y,2}}.
    {move,{x,1},{x,0}}.
    {move,{literal,"\n"},{x,1}}.
    {line,[{location,"fasta.erl",33}]}.
    {call_ext,2,{extfunc,erlang,'++',2}}.
    {test_heap,2,1}.
    {put_list,{integer,32},{x,0},{x,1}}.
    {move,{y,2},{x,0}}.
    {init_yregs,{list,[{y,2}]}}.
    {call_ext,2,{extfunc,erlang,'++',2}}.
    {test_heap,2,1}.
    {put_list,{integer,62},{x,0},{x,0}}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {move,{y,0},{x,1}}.
    {move,{y,1},{x,0}}.
    {call_last,2,{f,8},3}. % write_fasta_cycle/2


{function, write_fasta_cycle, 2, 8}.
  {label,7}.
    {line,[{location,"fasta.erl",35}]}.
    {func_info,{atom,fasta},{atom,write_fasta_cycle},2}.
  {label,8}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {test,is_ge,{f,9},[{integer,60},{tr,{x,1},{t_integer,any}}]}.
    {allocate,0,2}.
    {line,[{location,"fasta.erl",36}]}.
    {call,2,{f,13}}. % seq_len/2
    {test,is_tuple,{f,10},[{x,0}]}.
    {test,test_arity,{f,10},[{x,0},2]}.
    {get_tuple_element,{x,0},0,{x,0}}.
    {move,{literal,"\n"},{x,1}}.
    {line,[{location,"fasta.erl",37}]}.
    {call_ext,2,{extfunc,erlang,'++',2}}.
    {call_ext_last,1,{extfunc,io,put_chars,1},0}.
  {label,9}.
    {allocate,2,2}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,1},{y,1}}.
    {move,{integer,60},{x,1}}.
    {line,[{location,"fasta.erl",39}]}.
    {call,2,{f,13}}. % seq_len/2
    {test,is_tuple,{f,11},[{x,0}]}.
    {test,test_arity,{f,11},[{x,0},2]}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{y,0}}.
    {move,{x,1},{x,0}}.
    {move,{literal,"\n"},{x,1}}.
    {line,[{location,"fasta.erl",40}]}.
    {call_ext,2,{extfunc,erlang,'++',2}}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {line,[{location,"fasta.erl",41}]}.
    {gc_bif,'-',
            {f,0},
            0,
            [{tr,{y,1},{t_integer,{61,'+inf'}}},{integer,60}],
            {x,1}}.
    {move,{y,0},{x,0}}.
    {call_last,2,{f,8},2}. % write_fasta_cycle/2
  {label,10}.
    {line,[{location,"fasta.erl",36}]}.
    {badmatch,{x,0}}.
  {label,11}.
    {line,[{location,"fasta.erl",39}]}.
    {badmatch,{x,0}}.


{function, seq_len, 2, 13}.
  {label,12}.
    {line,[{location,"fasta.erl",44}]}.
    {func_info,{atom,fasta},{atom,seq_len},2}.
  {label,13}.
    {'%',{var_info,{x,1},[{type,{t_integer,{'-inf',60}}}]}}.
    {gc_bif,length,{f,15},2,[{x,0}],{x,2}}.
    {test,is_ge,
          {f,14},
          [{tr,{x,2},{t_integer,{0,288230376151711743}}},
           {tr,{x,1},{t_integer,{'-inf',60}}}]}.
    {swap,{x,0},{x,1}}.
    {line,[{location,"fasta.erl",45}]}.
    {call_ext_only,2,{extfunc,lists,split,2}}.
  {label,14}.
    {line,[{location,"fasta.erl",46}]}.
    {gc_bif,'-',
            {f,0},
            3,
            [{tr,{x,1},{t_integer,{1,60}}},{tr,{x,2},{t_integer,{0,59}}}],
            {x,2}}.
    {test,is_ge,{f,15},[{tr,{x,2},{t_integer,{-58,60}}},{integer,288}]}.
    {allocate,1,2}.
    {move,{x,1},{y,0}}.
    {move,{literal,"GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"},
          {x,1}}.
    {line,[{location,"fasta.erl",47}]}.
    {call_ext,2,{extfunc,erlang,'++',2}}.
    {move,{y,0},{x,1}}.
    {call_last,2,{f,13},1}. % seq_len/2
  {label,15}.
    {line,[{location,"fasta.erl",49}]}.
    {gc_bif,length,{f,0},2,[{x,0}],{x,2}}.
    {gc_bif,'-',
            {f,0},
            3,
            [{tr,{x,1},{t_integer,{'-inf',60}}},
             {tr,{x,2},{t_integer,{0,288230376151711743}}}],
            {x,1}}.
    {allocate,2,2}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,0},{y,1}}.
    {move,{x,1},{x,0}}.
    {move,{literal,"GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"},
          {x,1}}.
    {call_ext,2,{extfunc,lists,split,2}}.
    {move,{x,0},{y,0}}.
    {test,is_tuple,{f,16},[{x,0}]}.
    {test,test_arity,{f,16},[{x,0},2]}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {move,{y,1},{x,0}}.
    {init_yregs,{list,[{y,1}]}}.
    {line,[{location,"fasta.erl",50}]}.
    {call_ext,2,{extfunc,erlang,'++',2}}.
    {test_heap,3,1}.
    {get_tuple_element,{y,0},1,{x,1}}.
    {put_tuple2,{x,0},{list,[{x,0},{x,1}]}}.
    {deallocate,2}.
    return.
  {label,16}.
    {line,[{location,"fasta.erl",49}]}.
    {badmatch,{x,0}}.


{function, write_fasta_rand, 5, 18}.
  {label,17}.
    {line,[{location,"fasta.erl",53}]}.
    {func_info,{atom,fasta},{atom,write_fasta_rand},5}.
  {label,18}.
    {'%',{var_info,{x,0},[{type,{t_cons,{t_integer,{69,87}},nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_cons,{t_integer,{32,121}},nil}}]}}.
    {'%',
        {var_info,
            {x,2},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {'%',{var_info,{x,3},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,4},[{type,{t_integer,{0,139967}}}]}}.
    {allocate,4,5}.
    {move,{x,4},{y,0}}.
    {move,{x,3},{y,1}}.
    {move,{x,2},{y,2}}.
    {move,{x,0},{y,3}}.
    {move,{x,1},{x,0}}.
    {move,{literal,"\n"},{x,1}}.
    {line,[{location,"fasta.erl",54}]}.
    {call_ext,2,{extfunc,erlang,'++',2}}.
    {test_heap,2,1}.
    {put_list,{integer,32},{x,0},{x,1}}.
    {move,{y,3},{x,0}}.
    {init_yregs,{list,[{y,3}]}}.
    {call_ext,2,{extfunc,erlang,'++',2}}.
    {test_heap,2,1}.
    {put_list,{integer,62},{x,0},{x,0}}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {move,{y,0},{x,2}}.
    {move,{y,1},{x,1}}.
    {move,{y,2},{x,0}}.
    {call_last,3,{f,20},4}. % write_fasta_rand/3


{function, write_fasta_rand, 3, 20}.
  {label,19}.
    {line,[{location,"fasta.erl",57}]}.
    {func_info,{atom,fasta},{atom,write_fasta_rand},3}.
  {label,20}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_integer,{0,139967}}}]}}.
    {test,is_ge,{f,21},[{integer,60},{tr,{x,1},{t_integer,any}}]}.
    {allocate,1,3}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,2},{x,3}}.
    {move,nil,{x,2}}.
    {line,[{location,"fasta.erl",58}]}.
    {call,4,{f,23}}. % rand_list/4
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_tuple,2,true,
                     #{1 =>
                           {t_cons,
                               {t_union,none,
                                   {t_cons,{t_integer,{10,10}},nil},
                                   {t_integer,{66,116}},
                                   none,none,none},
                               nil},
                       2 => {t_integer,{0,139967}}}}}]}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{y,0}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"fasta.erl",59}]}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {move,{y,0},{x,0}}.
    {deallocate,1}.
    return.
  {label,21}.
    {allocate,3,3}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,1},{y,1}}.
    {move,{x,0},{y,2}}.
    {move,{x,2},{x,3}}.
    {move,nil,{x,2}}.
    {move,{integer,60},{x,1}}.
    {line,[{location,"fasta.erl",62}]}.
    {call,4,{f,23}}. % rand_list/4
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_tuple,2,true,
                     #{1 =>
                           {t_cons,
                               {t_union,none,
                                   {t_cons,{t_integer,{10,10}},nil},
                                   {t_integer,{66,116}},
                                   none,none,none},
                               nil},
                       2 => {t_integer,{0,139967}}}}}]}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {get_tuple_element,{x,0},1,{y,0}}.
    {move,{x,1},{x,0}}.
    {line,[{location,"fasta.erl",63}]}.
    {call_ext,1,{extfunc,io,put_chars,1}}.
    {line,[{location,"fasta.erl",64}]}.
    {gc_bif,'-',
            {f,0},
            0,
            [{tr,{y,1},{t_integer,{61,'+inf'}}},{integer,60}],
            {x,1}}.
    {move,{y,0},{x,2}}.
    {move,{y,2},{x,0}}.
    {call_last,3,{f,20},3}. % write_fasta_rand/3


{function, rand_list, 4, 23}.
  {label,22}.
    {line,[{location,"fasta.erl",67}]}.
    {func_info,{atom,fasta},{atom,rand_list},4}.
  {label,23}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_integer,any}}]}}.
    {'%',{var_info,{x,2},[{type,{t_list,{t_integer,{66,116}},nil}}]}}.
    {'%',{var_info,{x,3},[{type,{t_integer,{0,139967}}}]}}.
    {test,is_eq_exact,{f,24},[{tr,{x,1},{t_integer,any}},{integer,0}]}.
    {allocate_heap,1,2,4}.
    {move,{x,3},{y,0}}.
    {put_list,{literal,"\n"},{x,2},{x,0}}.
    {line,[{location,"fasta.erl",68}]}.
    {call_ext,1,{extfunc,lists,reverse,1}}.
    {test_heap,3,1}.
    {put_tuple2,{x,0},{list,[{x,0},{y,0}]}}.
    {deallocate,1}.
    return.
  {label,24}.
    {allocate,4,4}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,2},{y,1}}.
    {move,{x,1},{y,2}}.
    {move,{x,0},{y,3}}.
    {move,{x,3},{x,0}}.
    {line,[{location,"fasta.erl",70}]}.
    {call,1,{f,26}}. % rand/1
    {'%',{var_info,{x,0},
                   [{type,{t_tuple,2,true,
                                   #{1 => {t_float,any},
                                     2 => {t_integer,{0,139967}}}}}]}}.
    {get_tuple_element,{x,0},1,{y,0}}.
    {line,[{location,"fasta.erl",71}]}.
    {gc_bif,'-',{f,0},1,[{tr,{y,2},{t_integer,any}},{integer,1}],{y,2}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {move,{y,3},{x,0}}.
    {call,2,{f,28}}. % choose_base/2
    {'%',{var_info,{x,0},[{type,{t_integer,{66,116}}}]}}.
    {test_heap,2,1}.
    {put_list,{x,0},{y,1},{x,2}}.
    {move,{y,0},{x,3}}.
    {move,{y,2},{x,1}}.
    {move,{y,3},{x,0}}.
    {call_last,4,{f,23},4}. % rand_list/4


{function, rand, 1, 26}.
  {label,25}.
    {line,[{location,"fasta.erl",74}]}.
    {func_info,{atom,fasta},{atom,rand},1}.
  {label,26}.
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


{function, choose_base, 2, 28}.
  {label,27}.
    {line,[{location,"fasta.erl",79}]}.
    {func_info,{atom,fasta},{atom,choose_base},2}.
  {label,28}.
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
          {f,29},
          [{tr,{x,0},
               {t_list,{t_tuple,2,true,
                                #{1 => {t_integer,{66,116}},
                                  2 => {t_float,any}}},
                       nil}},
           nil]}.
    {get_tuple_element,{x,2},1,{x,2}}.
    {test,is_lt,{f,30},[{tr,{x,1},{t_float,any}},{tr,{x,2},{t_float,any}}]}.
  {label,29}.
    {move,{x,3},{x,0}}.
    return.
  {label,30}.
    {call_only,2,{f,28}}. % choose_base/2


{function, cumulative, 1, 32}.
  {label,31}.
    {line,[{location,"fasta.erl",87}]}.
    {func_info,{atom,fasta},{atom,cumulative},1}.
  {label,32}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},
                           2 => {t_float,{0.02,0.302954942668}}}},
                     nil}}]}}.
    {move,{float,0.0},{x,1}}.
    {call_only,2,{f,34}}. % cumulative/2


{function, cumulative, 2, 34}.
  {label,33}.
    {line,[{location,"fasta.erl",88}]}.
    {func_info,{atom,fasta},{atom,cumulative},2}.
  {label,34}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},
                           2 => {t_float,{0.02,0.302954942668}}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,any}}]}}.
    {test,is_nonempty_list,{f,35},[{x,0}]}.
    {get_list,{x,0},{x,2},{x,0}}.
    {get_tuple_element,{x,2},0,{x,3}}.
    {get_tuple_element,{x,2},1,{x,2}}.
    {fmove,{x,1},{fr,0}}.
    {fmove,{x,2},{fr,1}}.
    {line,[{location,"fasta.erl",90}]}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {allocate_heap,2,{alloc,[{words,0},{floats,1},{funs,0}]},4}.
    {move,{x,3},{y,1}}.
    {fmove,{fr,0},{y,0}}.
    {move,{y,0},{x,1}}.
    {line,[{location,"fasta.erl",91}]}.
    {call,2,{f,34}}. % cumulative/2
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,2,true,
                         #{1 => {t_integer,{66,116}},2 => {t_float,any}}},
                     nil}}]}}.
    {test_heap,5,1}.
    {put_tuple2,{x,1},{list,[{y,1},{y,0}]}}.
    {put_list,{x,1},{x,0},{x,0}}.
    {deallocate,2}.
    return.
  {label,35}.
    return.


{function, module_info, 0, 37}.
  {label,36}.
    {line,[]}.
    {func_info,{atom,fasta},{atom,module_info},0}.
  {label,37}.
    {move,{atom,fasta},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 39}.
  {label,38}.
    {line,[]}.
    {func_info,{atom,fasta},{atom,module_info},1}.
  {label,39}.
    {move,{x,0},{x,1}}.
    {move,{atom,fasta},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.
