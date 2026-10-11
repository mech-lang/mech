{module, nbody}.  %% version = 0

{exports, [{main,1},{module_info,0},{module_info,1}]}.

{attributes, []}.

{labels, 40}.


{function, main, 1, 2}.
  {label,1}.
    {line,[{location,"nbody.erl",15}]}.
    {func_info,{atom,nbody},{atom,main},1}.
  {label,2}.
    {test,is_nonempty_list,{f,3},[{x,0}]}.
    {get_list,{x,0},{x,1},{x,2}}.
    {test,is_nil,{f,3},[{x,2}]}.
    {allocate,0,2}.
    {move,{x,1},{x,0}}.
    {line,[{location,"nbody.erl",16}]}.
    {call_ext,1,{extfunc,erlang,list_to_integer,1}}.
    {line,[{location,"nbody.erl",17}]}.
    {call,1,{f,2}}. % main/1
    {move,{integer,0},{x,0}}.
    {line,[{location,"nbody.erl",18}]}.
    {call_ext_last,1,{extfunc,erlang,halt,1},0}.
  {label,3}.
    {allocate,2,1}.
    {init_yregs,{list,[{y,0}]}}.
    {move,{x,0},{y,1}}.
    {line,[{location,"nbody.erl",21}]}.
    {call,0,{f,28}}. % bodies/0
    {'%',
     {var_info,
      {x,0},
      [{type,
        {t_cons,
         {t_union,none,none,none,
          [{{7,{t_float,{0.0,0.0}}},
            {t_tuple,7,true,
             #{1 => {t_float,{0.0,0.0}},
               2 => {t_float,{0.0,0.0}},
               3 => {t_float,{0.0,0.0}},
               4 => {t_float,{0.0,0.0}},
               5 => {t_float,{0.0,0.0}},
               6 => {t_float,{0.0,0.0}},
               7 => {t_float,{39.47841760435743,39.47841760435743}}}}},
           {{7,{t_float,{4.841431442464721,4.841431442464721}}},
            {t_tuple,7,true,
             #{1 => {t_float,{4.841431442464721,4.841431442464721}},
               2 => {t_float,{-1.1603200440274284,-1.1603200440274284}},
               3 => {t_float,{-0.10362204447112311,-0.10362204447112311}},
               4 => {t_float,{0.606326392995832,0.606326392995832}},
               5 => {t_float,{2.81198684491626,2.81198684491626}},
               6 => {t_float,{-0.02521836165988763,-0.02521836165988763}},
               7 => {t_float,{0.03769367487038949,0.03769367487038949}}}}},
           {{7,{t_float,{8.34336671824458,8.34336671824458}}},
            {t_tuple,7,true,
             #{1 => {t_float,{8.34336671824458,8.34336671824458}},
               2 => {t_float,{4.124798564124305,4.124798564124305}},
               3 => {t_float,{-0.4035234171143214,-0.4035234171143214}},
               4 => {t_float,{-1.0107743461787924,-1.0107743461787924}},
               5 => {t_float,{1.8256623712304119,1.8256623712304119}},
               6 => {t_float,{0.008415761376584154,0.008415761376584154}},
               7 => {t_float,{0.011286326131968767,0.011286326131968767}}}}},
           {{7,{t_float,{12.894369562139131,12.894369562139131}}},
            {t_tuple,7,true,
             #{1 => {t_float,{12.894369562139131,12.894369562139131}},
               2 => {t_float,{-15.111151401698631,-15.111151401698631}},
               3 => {t_float,{-0.22330757889265573,-0.22330757889265573}},
               4 => {t_float,{1.0827910064415354,1.0827910064415354}},
               5 => {t_float,{0.8687130181696082,0.8687130181696082}},
               6 => {t_float,{-0.010832637401363636,-0.010832637401363636}},
               7 => {t_float,{0.0017237240570597112,0.0017237240570597112}}}}},
           {{7,{t_float,{15.379697114850917,15.379697114850917}}},
            {t_tuple,7,true,
             #{1 => {t_float,{15.379697114850917,15.379697114850917}},
               2 => {t_float,{-25.919314609987964,-25.919314609987964}},
               3 => {t_float,{0.17925877295037118,0.17925877295037118}},
               4 => {t_float,{0.979090732243898,0.979090732243898}},
               5 => {t_float,{0.5946989986476762,0.5946989986476762}},
               6 => {t_float,{-0.034755955504078104,-0.034755955504078104}},
               7 =>
                {t_float,{0.0020336868699246304,0.0020336868699246304}}}}}],
          none,none},
         nil}}]}}.
    {call,1,{f,5}}. % offset_momentum/1
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_cons,
                     {t_tuple,7,true,
                         #{1 => {t_float,{0.0,15.379697114850917}},
                           2 =>
                               {t_float,
                                   {-25.919314609987964,4.124798564124305}},
                           3 =>
                               {t_float,
                                   {-0.4035234171143214,0.17925877295037118}},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {move,{x,0},{y,0}}.
    {line,[{location,"nbody.erl",22}]}.
    {call,1,{f,7}}. % energy/1
    {'%',{var_info,{x,0},[{type,{t_float,any}}]}}.
    {test_heap,2,1}.
    {put_list,{x,0},nil,{x,1}}.
    {move,{literal,"~.9f\n"},{x,0}}.
    {call_ext,2,{extfunc,io,format,2}}.
    {move,{y,0},{x,1}}.
    {move,{float,0.01},{x,2}}.
    {move,{y,1},{x,0}}.
    {trim,2,0}.
    {line,[{location,"nbody.erl",23}]}.
    {call,3,{f,12}}. % advance/3
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {call,1,{f,7}}. % energy/1
    {'%',{var_info,{x,0},[{type,{t_float,any}}]}}.
    {test_heap,2,1}.
    {put_list,{x,0},nil,{x,1}}.
    {move,{literal,"~.9f\n"},{x,0}}.
    {call_ext_last,2,{extfunc,io,format,2},0}.


{function, offset_momentum, 1, 5}.
  {label,4}.
    {line,[{location,"nbody.erl",25}]}.
    {func_info,{atom,nbody},{atom,offset_momentum},1}.
  {label,5}.
    {'%',
     {var_info,
      {x,0},
      [{type,
        {t_cons,
         {t_union,none,none,none,
          [{{7,{t_float,{0.0,0.0}}},
            {t_tuple,7,true,
             #{1 => {t_float,{0.0,0.0}},
               2 => {t_float,{0.0,0.0}},
               3 => {t_float,{0.0,0.0}},
               4 => {t_float,{0.0,0.0}},
               5 => {t_float,{0.0,0.0}},
               6 => {t_float,{0.0,0.0}},
               7 => {t_float,{39.47841760435743,39.47841760435743}}}}},
           {{7,{t_float,{4.841431442464721,4.841431442464721}}},
            {t_tuple,7,true,
             #{1 => {t_float,{4.841431442464721,4.841431442464721}},
               2 => {t_float,{-1.1603200440274284,-1.1603200440274284}},
               3 => {t_float,{-0.10362204447112311,-0.10362204447112311}},
               4 => {t_float,{0.606326392995832,0.606326392995832}},
               5 => {t_float,{2.81198684491626,2.81198684491626}},
               6 => {t_float,{-0.02521836165988763,-0.02521836165988763}},
               7 => {t_float,{0.03769367487038949,0.03769367487038949}}}}},
           {{7,{t_float,{8.34336671824458,8.34336671824458}}},
            {t_tuple,7,true,
             #{1 => {t_float,{8.34336671824458,8.34336671824458}},
               2 => {t_float,{4.124798564124305,4.124798564124305}},
               3 => {t_float,{-0.4035234171143214,-0.4035234171143214}},
               4 => {t_float,{-1.0107743461787924,-1.0107743461787924}},
               5 => {t_float,{1.8256623712304119,1.8256623712304119}},
               6 => {t_float,{0.008415761376584154,0.008415761376584154}},
               7 => {t_float,{0.011286326131968767,0.011286326131968767}}}}},
           {{7,{t_float,{12.894369562139131,12.894369562139131}}},
            {t_tuple,7,true,
             #{1 => {t_float,{12.894369562139131,12.894369562139131}},
               2 => {t_float,{-15.111151401698631,-15.111151401698631}},
               3 => {t_float,{-0.22330757889265573,-0.22330757889265573}},
               4 => {t_float,{1.0827910064415354,1.0827910064415354}},
               5 => {t_float,{0.8687130181696082,0.8687130181696082}},
               6 => {t_float,{-0.010832637401363636,-0.010832637401363636}},
               7 => {t_float,{0.0017237240570597112,0.0017237240570597112}}}}},
           {{7,{t_float,{15.379697114850917,15.379697114850917}}},
            {t_tuple,7,true,
             #{1 => {t_float,{15.379697114850917,15.379697114850917}},
               2 => {t_float,{-25.919314609987964,-25.919314609987964}},
               3 => {t_float,{0.17925877295037118,0.17925877295037118}},
               4 => {t_float,{0.979090732243898,0.979090732243898}},
               5 => {t_float,{0.5946989986476762,0.5946989986476762}},
               6 => {t_float,{-0.034755955504078104,-0.034755955504078104}},
               7 =>
                {t_float,{0.0020336868699246304,0.0020336868699246304}}}}}],
          none,none},
         nil}}]}}.
    {allocate_heap,2,{alloc,[{words,0},{floats,0},{funs,1}]},1}.
    {get_list,{x,0},{y,1},{y,0}}.
    {make_fun3,{f,37},0,0,{x,1},{list,[]}}.
    {'%',
        {var_info,
            {x,1},
            [{fun_type,
                 {t_tuple,3,true,
                     #{1 => {t_float,any},
                       2 => {t_float,any},
                       3 => {t_float,any}}}}]}}.
    {move,{x,0},{x,2}}.
    {move,{x,1},{x,0}}.
    {move,{literal,{0.0,0.0,0.0}},{x,1}}.
    {line,[{location,"nbody.erl",27}]}.
    {call_ext,3,{extfunc,lists,foldl,3}}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {fmove,{x,1},{fr,0}}.
    {line,[{location,"nbody.erl",34}]}.
    {bif,fnegate,{f,0},[{fr,0}],{fr,0}}.
    {fmove,{float,39.47841760435743},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {get_tuple_element,{x,0},1,{x,1}}.
    {fmove,{x,1},{fr,2}}.
    {bif,fnegate,{f,0},[{fr,2}],{fr,2}}.
    {bif,fdiv,{f,0},[{fr,2},{fr,1}],{fr,2}}.
    {get_tuple_element,{x,0},2,{x,0}}.
    {fmove,{x,0},{fr,3}}.
    {bif,fnegate,{f,0},[{fr,3}],{fr,3}}.
    {bif,fdiv,{f,0},[{fr,3},{fr,1}],{fr,1}}.
    {test_heap,{alloc,[{words,10},{floats,3},{funs,0}]},0}.
    {fmove,{fr,0},{x,0}}.
    {fmove,{fr,2},{x,1}}.
    {fmove,{fr,1},{x,2}}.
    {get_tuple_element,{y,1},0,{x,3}}.
    {get_tuple_element,{y,1},1,{x,4}}.
    {get_tuple_element,{y,1},2,{x,5}}.
    {get_tuple_element,{y,1},6,{x,6}}.
    {put_tuple2,{x,0},{list,[{x,3},{x,4},{x,5},{x,0},{x,1},{x,2},{x,6}]}}.
    {put_list,{x,0},{y,0},{x,0}}.
    {deallocate,2}.
    return.


{function, energy, 1, 7}.
  {label,6}.
    {line,[{location,"nbody.erl",36}]}.
    {func_info,{atom,nbody},{atom,energy},1}.
  {label,7}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {move,{float,0.0},{x,1}}.
    {call_only,2,{f,9}}. % energy/2


{function, energy, 2, 9}.
  {label,8}.
    {line,[{location,"nbody.erl",38}]}.
    {func_info,{atom,nbody},{atom,energy},2}.
  {label,9}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,any}}]}}.
    {test,is_nonempty_list,{f,10},[{x,0}]}.
    {test_heap,{alloc,[{words,4},{floats,0},{funs,1}]},2}.
    {get_list,{x,0},{x,2},{x,0}}.
    {get_tuple_element,{x,2},0,{x,3}}.
    {get_tuple_element,{x,2},1,{x,4}}.
    {get_tuple_element,{x,2},2,{x,5}}.
    {get_tuple_element,{x,2},6,{x,6}}.
    {make_fun3,{f,34},0,0,{x,3},{list,[{x,6},{x,5},{x,4},{x,3}]}}.
    {'%',{var_info,{x,3},[{fun_type,{t_float,any}}]}}.
    {fmove,{x,6},{fr,0}}.
    {fmove,{float,0.5},{fr,1}}.
    {line,[{location,"nbody.erl",49}]}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {get_tuple_element,{x,2},3,{x,4}}.
    {fmove,{x,4},{fr,1}}.
    {bif,fmul,{f,0},[{fr,1},{fr,1}],{fr,1}}.
    {get_tuple_element,{x,2},4,{x,4}}.
    {fmove,{x,4},{fr,2}}.
    {bif,fmul,{f,0},[{fr,2},{fr,2}],{fr,2}}.
    {bif,fadd,{f,0},[{fr,1},{fr,2}],{fr,1}}.
    {get_tuple_element,{x,2},5,{x,2}}.
    {fmove,{x,2},{fr,2}}.
    {bif,fmul,{f,0},[{fr,2},{fr,2}],{fr,2}}.
    {bif,fadd,{f,0},[{fr,1},{fr,2}],{fr,1}}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{x,1},{fr,1}}.
    {bif,fadd,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {allocate_heap,1,{alloc,[{words,0},{floats,1},{funs,0}]},4}.
    {move,{x,0},{y,0}}.
    {fmove,{fr,0},{x,1}}.
    {move,{x,0},{x,2}}.
    {move,{x,3},{x,0}}.
    {line,[{location,"nbody.erl",41}]}.
    {call_ext,3,{extfunc,lists,foldl,3}}.
    {move,{x,0},{x,1}}.
    {move,{y,0},{x,0}}.
    {call_last,2,{f,9},1}. % energy/2
  {label,10}.
    {move,{x,1},{x,0}}.
    return.


{function, advance, 3, 12}.
  {label,11}.
    {line,[{location,"nbody.erl",52}]}.
    {func_info,{atom,nbody},{atom,advance},3}.
  {label,12}.
    {'%',
        {var_info,
            {x,1},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {'%',{var_info,{x,2},[{type,{t_float,{0.01,0.01}}}]}}.
    {test,is_eq_exact,{f,13},[{x,0},{integer,0}]}.
    {move,{x,1},{x,0}}.
    return.
  {label,13}.
    {line,[{location,"nbody.erl",53}]}.
    {gc_bif,'-',{f,0},2,[{x,0},{integer,1}],{x,0}}.
    {allocate,1,2}.
    {move,{x,0},{y,0}}.
    {move,{x,1},{x,0}}.
    {move,{float,0.01},{x,1}}.
    {call,2,{f,15}}. % adv1/2
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {move,{float,0.01},{x,1}}.
    {call,2,{f,23}}. % adv2/2
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {move,{x,0},{x,1}}.
    {move,{float,0.01},{x,2}}.
    {move,{y,0},{x,0}}.
    {call_last,3,{f,12},1}. % advance/3


{function, adv1, 2, 15}.
  {label,14}.
    {line,[{location,"nbody.erl",59}]}.
    {func_info,{atom,nbody},{atom,adv1},2}.
  {label,15}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,{0.01,0.01}}}]}}.
    {move,nil,{x,2}}.
    {call_only,3,{f,17}}. % adv1/3


{function, adv1, 3, 17}.
  {label,16}.
    {line,[{location,"nbody.erl",61}]}.
    {func_info,{atom,nbody},{atom,adv1},3}.
  {label,17}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,{0.01,0.01}}}]}}.
    {'%',
        {var_info,
            {x,2},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {test,is_nonempty_list,{f,18},[{x,0}]}.
    {allocate,1,3}.
    {move,{x,2},{y,0}}.
    {get_list,{x,0},{x,1},{x,0}}.
    {move,{float,0.01},{x,3}}.
    {move,nil,{x,2}}.
    {swap,{x,0},{x,1}}.
    {line,[{location,"nbody.erl",63}]}.
    {call,4,{f,20}}. % adv1/4
    {'%',
     {var_info,
      {x,0},
      [{type,
        {t_tuple,2,true,
         #{1 =>
            {t_tuple,7,true,
             #{1 => {t_float,any},
               2 => {t_float,any},
               3 => {t_float,any},
               4 => {t_float,any},
               5 => {t_float,any},
               6 => {t_float,any},
               7 => {t_float,{0.0017237240570597112,39.47841760435743}}}},
           2 =>
            {t_list,
             {t_tuple,7,true,
              #{1 => {t_float,any},
                2 => {t_float,any},
                3 => {t_float,any},
                4 => {t_float,any},
                5 => {t_float,any},
                6 => {t_float,any},
                7 => {t_float,{0.0017237240570597112,39.47841760435743}}}},
             nil}}}}]}}.
    {test_heap,2,1}.
    {get_tuple_element,{x,0},0,{x,1}}.
    {put_list,{x,1},{y,0},{x,2}}.
    {get_tuple_element,{x,0},1,{x,0}}.
    {move,{float,0.01},{x,1}}.
    {call_last,3,{f,17},1}. % adv1/3
  {label,18}.
    {move,{x,2},{x,0}}.
    return.


{function, adv1, 4, 20}.
  {label,19}.
    {line,[{location,"nbody.erl",67}]}.
    {func_info,{atom,nbody},{atom,adv1},4}.
  {label,20}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_tuple,7,true,
                     #{1 => {t_float,any},
                       2 => {t_float,any},
                       3 => {t_float,any},
                       4 => {t_float,any},
                       5 => {t_float,any},
                       6 => {t_float,any},
                       7 =>
                           {t_float,
                               {0.0017237240570597112,
                                39.47841760435743}}}}}]}}.
    {'%',
        {var_info,
            {x,1},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {'%',
        {var_info,
            {x,2},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {'%',{var_info,{x,3},[{type,{t_float,{0.01,0.01}}}]}}.
    {test,is_nil,
        {f,21},
        [{tr,{x,1},
             {t_list,
                 {t_tuple,7,true,
                     #{1 => {t_float,any},
                       2 => {t_float,any},
                       3 => {t_float,any},
                       4 => {t_float,any},
                       5 => {t_float,any},
                       6 => {t_float,any},
                       7 =>
                           {t_float,
                               {0.0017237240570597112,39.47841760435743}}}},
                 nil}}]}.
    {test_heap,3,3}.
    {put_tuple2,{x,0},{list,[{x,0},{x,2}]}}.
    return.
  {label,21}.
    {get_list,{x,1},{x,3},{x,1}}.
    {get_tuple_element,{x,3},0,{x,4}}.
    {get_tuple_element,{x,0},0,{x,5}}.
    {fmove,{x,5},{fr,0}}.
    {fmove,{x,4},{fr,1}}.
    {line,[{location,"nbody.erl",71}]}.
    {bif,fsub,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {get_tuple_element,{x,3},1,{x,6}}.
    {get_tuple_element,{x,0},1,{x,7}}.
    {fmove,{x,7},{fr,1}}.
    {fmove,{x,6},{fr,2}}.
    {line,[{location,"nbody.erl",72}]}.
    {bif,fsub,{f,0},[{fr,1},{fr,2}],{fr,1}}.
    {get_tuple_element,{x,3},2,{x,8}}.
    {get_tuple_element,{x,0},2,{x,9}}.
    {fmove,{x,9},{fr,2}}.
    {fmove,{x,8},{fr,3}}.
    {line,[{location,"nbody.erl",73}]}.
    {bif,fsub,{f,0},[{fr,2},{fr,3}],{fr,2}}.
    {line,[{location,"nbody.erl",74}]}.
    {bif,fmul,{f,0},[{fr,0},{fr,0}],{fr,3}}.
    {bif,fmul,{f,0},[{fr,1},{fr,1}],{fr,4}}.
    {bif,fadd,{f,0},[{fr,3},{fr,4}],{fr,3}}.
    {bif,fmul,{f,0},[{fr,2},{fr,2}],{fr,4}}.
    {bif,fadd,{f,0},[{fr,3},{fr,4}],{fr,3}}.
    {allocate_heap,13,{alloc,[{words,0},{floats,4},{funs,0}]},10}.
    {move,{x,4},{y,3}}.
    {move,{x,6},{y,4}}.
    {move,{x,8},{y,5}}.
    {move,{x,3},{y,6}}.
    {move,{x,1},{y,7}}.
    {move,{x,5},{y,8}}.
    {move,{x,7},{y,9}}.
    {move,{x,9},{y,10}}.
    {move,{x,2},{y,11}}.
    {move,{x,0},{y,12}}.
    {fmove,{fr,2},{y,2}}.
    {fmove,{fr,1},{y,1}}.
    {fmove,{fr,0},{y,0}}.
    {fmove,{fr,3},{x,0}}.
    {call_ext,1,{extfunc,math,sqrt,1}}.
    {fmove,{x,0},{fr,0}}.
    {line,[{location,"nbody.erl",75}]}.
    {bif,fmul,{f,0},[{fr,0},{fr,0}],{fr,1}}.
    {bif,fmul,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {fmove,{float,0.01},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {get_tuple_element,{y,12},6,{x,0}}.
    {fmove,{x,0},{fr,1}}.
    {line,[{location,"nbody.erl",76}]}.
    {bif,fmul,{f,0},[{fr,1},{fr,0}],{fr,1}}.
    {get_tuple_element,{y,6},6,{x,1}}.
    {fmove,{x,1},{fr,2}}.
    {line,[{location,"nbody.erl",77}]}.
    {bif,fmul,{f,0},[{fr,2},{fr,0}],{fr,0}}.
    {fmove,{y,0},{fr,2}}.
    {line,[{location,"nbody.erl",78}]}.
    {bif,fmul,{f,0},[{fr,2},{fr,0}],{fr,2}}.
    {get_tuple_element,{y,12},3,{x,2}}.
    {fmove,{x,2},{fr,3}}.
    {bif,fsub,{f,0},[{fr,3},{fr,2}],{fr,2}}.
    {fmove,{y,1},{fr,3}}.
    {bif,fmul,{f,0},[{fr,3},{fr,0}],{fr,3}}.
    {get_tuple_element,{y,12},4,{x,2}}.
    {fmove,{x,2},{fr,4}}.
    {bif,fsub,{f,0},[{fr,4},{fr,3}],{fr,3}}.
    {fmove,{y,2},{fr,4}}.
    {bif,fmul,{f,0},[{fr,4},{fr,0}],{fr,0}}.
    {get_tuple_element,{y,12},5,{x,2}}.
    {fmove,{x,2},{fr,4}}.
    {bif,fsub,{f,0},[{fr,4},{fr,0}],{fr,0}}.
    {test_heap,{alloc,[{words,8},{floats,4},{funs,0}]},2}.
    {fmove,{fr,1},{x,2}}.
    {fmove,{fr,2},{x,3}}.
    {fmove,{fr,3},{x,4}}.
    {fmove,{fr,0},{x,5}}.
    {put_tuple2,{x,0},{list,[{y,8},{y,9},{y,10},{x,3},{x,4},{x,5},{x,0}]}}.
    {fmove,{y,0},{fr,0}}.
    {fmove,{x,2},{fr,1}}.
    {line,[{location,"nbody.erl",79}]}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {get_tuple_element,{y,6},3,{x,2}}.
    {fmove,{x,2},{fr,2}}.
    {bif,fadd,{f,0},[{fr,2},{fr,0}],{fr,0}}.
    {fmove,{y,1},{fr,2}}.
    {bif,fmul,{f,0},[{fr,2},{fr,1}],{fr,2}}.
    {get_tuple_element,{y,6},4,{x,2}}.
    {fmove,{x,2},{fr,3}}.
    {bif,fadd,{f,0},[{fr,3},{fr,2}],{fr,2}}.
    {fmove,{y,2},{fr,3}}.
    {bif,fmul,{f,0},[{fr,3},{fr,1}],{fr,1}}.
    {get_tuple_element,{y,6},5,{x,2}}.
    {fmove,{x,2},{fr,3}}.
    {bif,fadd,{f,0},[{fr,3},{fr,1}],{fr,1}}.
    {test_heap,{alloc,[{words,10},{floats,3},{funs,0}]},2}.
    {fmove,{fr,0},{x,2}}.
    {fmove,{fr,2},{x,3}}.
    {fmove,{fr,1},{x,4}}.
    {put_tuple2,{x,1},{list,[{y,3},{y,4},{y,5},{x,2},{x,3},{x,4},{x,1}]}}.
    {put_list,{x,1},{y,11},{x,2}}.
    {move,{float,0.01},{x,3}}.
    {move,{y,7},{x,1}}.
    {call_last,4,{f,20},13}. % adv1/4


{function, adv2, 2, 23}.
  {label,22}.
    {line,[{location,"nbody.erl",82}]}.
    {func_info,{atom,nbody},{atom,adv2},2}.
  {label,23}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,{0.01,0.01}}}]}}.
    {move,nil,{x,2}}.
    {call_only,3,{f,25}}. % adv2/3


{function, adv2, 3, 25}.
  {label,24}.
    {line,[{location,"nbody.erl",84}]}.
    {func_info,{atom,nbody},{atom,adv2},3}.
  {label,25}.
    {'%',
        {var_info,
            {x,0},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {'%',{var_info,{x,1},[{type,{t_float,{0.01,0.01}}}]}}.
    {'%',
        {var_info,
            {x,2},
            [{type,
                 {t_list,
                     {t_tuple,7,true,
                         #{1 => {t_float,any},
                           2 => {t_float,any},
                           3 => {t_float,any},
                           4 => {t_float,any},
                           5 => {t_float,any},
                           6 => {t_float,any},
                           7 =>
                               {t_float,
                                   {0.0017237240570597112,
                                    39.47841760435743}}}},
                     nil}}]}}.
    {test,is_nonempty_list,{f,26},[{x,0}]}.
    {get_list,{x,0},{x,3},{x,0}}.
    {get_tuple_element,{x,3},3,{x,4}}.
    {fmove,{x,4},{fr,0}}.
    {fmove,{float,0.01},{fr,1}}.
    {line,[{location,"nbody.erl",87}]}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {get_tuple_element,{x,3},0,{x,5}}.
    {fmove,{x,5},{fr,2}}.
    {bif,fadd,{f,0},[{fr,2},{fr,0}],{fr,0}}.
    {get_tuple_element,{x,3},4,{x,5}}.
    {fmove,{x,5},{fr,2}}.
    {bif,fmul,{f,0},[{fr,2},{fr,1}],{fr,2}}.
    {get_tuple_element,{x,3},1,{x,6}}.
    {fmove,{x,6},{fr,3}}.
    {bif,fadd,{f,0},[{fr,3},{fr,2}],{fr,2}}.
    {get_tuple_element,{x,3},5,{x,6}}.
    {fmove,{x,6},{fr,3}}.
    {bif,fmul,{f,0},[{fr,3},{fr,1}],{fr,1}}.
    {get_tuple_element,{x,3},2,{x,7}}.
    {fmove,{x,7},{fr,3}}.
    {bif,fadd,{f,0},[{fr,3},{fr,1}],{fr,1}}.
    {test_heap,{alloc,[{words,10},{floats,3},{funs,0}]},7}.
    {fmove,{fr,0},{x,7}}.
    {fmove,{fr,2},{x,8}}.
    {fmove,{fr,1},{x,9}}.
    {get_tuple_element,{x,3},6,{x,3}}.
    {put_tuple2,{x,3},{list,[{x,7},{x,8},{x,9},{x,4},{x,5},{x,6},{x,3}]}}.
    {put_list,{x,3},{x,2},{x,2}}.
    {call_only,3,{f,25}}. % adv2/3
  {label,26}.
    {move,{x,2},{x,0}}.
    return.


{function, bodies, 0, 28}.
  {label,27}.
    {line,[{location,"nbody.erl",89}]}.
    {func_info,{atom,nbody},{atom,bodies},0}.
  {label,28}.
    {move,{literal,[{0.0,0.0,0.0,0.0,0.0,0.0,39.47841760435743},
                    {4.841431442464721,-1.1603200440274284,
                     -0.10362204447112311,0.606326392995832,2.81198684491626,
                     -0.02521836165988763,0.03769367487038949},
                    {8.34336671824458,4.124798564124305,-0.4035234171143214,
                     -1.0107743461787924,1.8256623712304119,
                     0.008415761376584154,0.011286326131968767},
                    {12.894369562139131,-15.111151401698631,
                     -0.22330757889265573,1.0827910064415354,
                     0.8687130181696082,-0.010832637401363636,
                     0.0017237240570597112},
                    {15.379697114850917,-25.919314609987964,
                     0.17925877295037118,0.979090732243898,0.5946989986476762,
                     -0.034755955504078104,0.0020336868699246304}]},
          {x,0}}.
    return.


{function, module_info, 0, 30}.
  {label,29}.
    {line,[]}.
    {func_info,{atom,nbody},{atom,module_info},0}.
  {label,30}.
    {move,{atom,nbody},{x,0}}.
    {call_ext_only,1,{extfunc,erlang,get_module_info,1}}.


{function, module_info, 1, 32}.
  {label,31}.
    {line,[]}.
    {func_info,{atom,nbody},{atom,module_info},1}.
  {label,32}.
    {move,{x,0},{x,1}}.
    {move,{atom,nbody},{x,0}}.
    {call_ext_only,2,{extfunc,erlang,get_module_info,2}}.


{function, '-energy/2-fun-0-', 6, 34}.
  {label,33}.
    {line,[{location,"nbody.erl",41}]}.
    {func_info,{atom,nbody},{atom,'-energy/2-fun-0-'},6}.
  {label,34}.
    {'%',{var_info,{x,2},
                   [{type,{t_float,{0.0017237240570597112,
                                    39.47841760435743}}}]}}.
    {'%',{var_info,{x,3},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,4},[{type,{t_float,any}}]}}.
    {'%',{var_info,{x,5},[{type,{t_float,any}}]}}.
    {test,is_tuple,{f,35},[{x,0}]}.
    {test,test_arity,{f,35},[{x,0},7]}.
    {get_tuple_element,{x,0},0,{x,6}}.
    {test,is_float,{f,35},[{x,6}]}.
    {get_tuple_element,{x,0},1,{x,7}}.
    {test,is_float,{f,35},[{x,7}]}.
    {get_tuple_element,{x,0},2,{x,8}}.
    {test,is_float,{f,35},[{x,8}]}.
    {get_tuple_element,{x,0},6,{x,9}}.
    {test,is_float,{f,35},[{x,9}]}.
    {test,is_float,{f,35},[{x,1}]}.
    {fmove,{x,5},{fr,0}}.
    {fmove,{x,6},{fr,1}}.
    {line,[{location,"nbody.erl",43}]}.
    {bif,fsub,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{x,4},{fr,1}}.
    {fmove,{x,7},{fr,2}}.
    {line,[{location,"nbody.erl",44}]}.
    {bif,fsub,{f,0},[{fr,1},{fr,2}],{fr,1}}.
    {fmove,{x,3},{fr,2}}.
    {fmove,{x,8},{fr,3}}.
    {line,[{location,"nbody.erl",45}]}.
    {bif,fsub,{f,0},[{fr,2},{fr,3}],{fr,2}}.
    {line,[{location,"nbody.erl",46}]}.
    {bif,fmul,{f,0},[{fr,0},{fr,0}],{fr,0}}.
    {bif,fmul,{f,0},[{fr,1},{fr,1}],{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {bif,fmul,{f,0},[{fr,2},{fr,2}],{fr,1}}.
    {bif,fadd,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {allocate_heap,3,{alloc,[{words,0},{floats,1},{funs,0}]},10}.
    {move,{x,9},{y,0}}.
    {move,{x,2},{y,1}}.
    {move,{x,1},{y,2}}.
    {fmove,{fr,0},{x,0}}.
    {call_ext,1,{extfunc,math,sqrt,1}}.
    {fmove,{y,1},{fr,0}}.
    {fmove,{y,0},{fr,1}}.
    {line,[{location,"nbody.erl",47}]}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{x,0},{fr,1}}.
    {bif,fdiv,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{y,2},{fr,1}}.
    {bif,fsub,{f,0},[{fr,1},{fr,0}],{fr,0}}.
    {test_heap,{alloc,[{words,0},{floats,1},{funs,0}]},0}.
    {fmove,{fr,0},{x,0}}.
    {deallocate,3}.
    return.
  {label,35}.
    {call_only,2,{f,39}}. % '-energy/2-inlined-0-'/2


{function, '-offset_momentum/1-fun-0-', 2, 37}.
  {label,36}.
    {line,[{location,"nbody.erl",27}]}.
    {func_info,{atom,nbody},{atom,'-offset_momentum/1-fun-0-'},2}.
  {label,37}.
    {test,is_tuple,{f,36},[{x,0}]}.
    {test,test_arity,{f,36},[{x,0},7]}.
    {test,is_tuple,{f,36},[{x,1}]}.
    {test,test_arity,{f,36},[{x,1},3]}.
    {get_tuple_element,{x,0},3,{x,2}}.
    {test,is_float,{f,36},[{x,2}]}.
    {get_tuple_element,{x,0},4,{x,3}}.
    {test,is_float,{f,36},[{x,3}]}.
    {get_tuple_element,{x,0},6,{x,4}}.
    {test,is_float,{f,36},[{x,4}]}.
    {get_tuple_element,{x,1},0,{x,5}}.
    {test,is_float,{f,36},[{x,5}]}.
    {get_tuple_element,{x,1},1,{x,6}}.
    {test,is_float,{f,36},[{x,6}]}.
    {get_tuple_element,{x,1},2,{x,7}}.
    {test,is_float,{f,36},[{x,7}]}.
    {fmove,{x,2},{fr,0}}.
    {fmove,{x,4},{fr,1}}.
    {line,[{location,"nbody.erl",30}]}.
    {bif,fmul,{f,0},[{fr,0},{fr,1}],{fr,0}}.
    {fmove,{x,5},{fr,2}}.
    {bif,fadd,{f,0},[{fr,2},{fr,0}],{fr,0}}.
    {fmove,{x,3},{fr,2}}.
    {bif,fmul,{f,0},[{fr,2},{fr,1}],{fr,2}}.
    {fmove,{x,6},{fr,3}}.
    {bif,fadd,{f,0},[{fr,3},{fr,2}],{fr,2}}.
    {get_tuple_element,{x,0},5,{x,0}}.
    {fconv,{x,0},{fr,3}}.
    {bif,fmul,{f,0},[{fr,3},{fr,1}],{fr,1}}.
    {fmove,{x,7},{fr,3}}.
    {bif,fadd,{f,0},[{fr,3},{fr,1}],{fr,1}}.
    {test_heap,{alloc,[{words,4},{floats,3},{funs,0}]},0}.
    {fmove,{fr,0},{x,0}}.
    {fmove,{fr,2},{x,1}}.
    {fmove,{fr,1},{x,2}}.
    {put_tuple2,{x,0},{list,[{x,0},{x,1},{x,2}]}}.
    return.


{function, '-energy/2-inlined-0-', 2, 39}.
  {label,38}.
    {line,[{location,"nbody.erl",41}]}.
    {func_info,{atom,nbody},{atom,'-energy/2-inlined-0-'},2}.
  {label,39}.
    {jump,{f,38}}.
