/ Thin bindings to ordinary libraries; no benchmark computation.
lib:`"/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/array-comparison/all-20261009/stdlib/libgrowler-standard.dylib"
znew:lib 2:(`"k_z_new";1)
zadd:lib 2:(`"k_z_add";3)
zsub:lib 2:(`"k_z_sub";3)
zmul:lib 2:(`"k_z_mul";3)
zdiv:lib 2:(`"k_z_div";3)
zui:lib 2:(`"k_z_ui";1)
zcmp:lib 2:(`"k_z_cmp";2)
mapnew:lib 2:(`"k_map_new";1)
mapget:lib 2:(`"k_map_get";2)
mapset:lib 2:(`"k_map_set";3)
mapkeys:lib 2:(`"k_map_keys";1)
mapclose:lib 2:(`"k_map_close";1)
rxnew:lib 2:(`"k_regex_new";1)
rxcount:lib 2:(`"k_regex_count";2)
rxreplace:lib 2:(`"k_regex_replace";3)

procnew:lib 2:(`"k_process_new";1)
procpid:lib 2:(`"k_process_pid";1)
procfd:lib 2:(`"k_process_fd";1)
procwait:lib 2:(`"k_process_wait";1)
procexit:lib 2:(`"k_process_exit";1)
/ Generic process map, at most eight children. Callbacks run in K.
pfn:{x}
parallel:{[f;items]
 pfn::f
 workers:8&#items
 cuts:_(#items)*(!workers+1)%workers
 parts:{[items;a;b](b-a)#a_items}[items]'[(-1_cuts);(1_cuts)]
 launch:{[part]
  child:procnew 0
  $[0=procpid child;{[part]`0:`k@pfn'part;procexit 0}[part];child]}
 children:launch'parts
 collect:{[h]
  lines:0:procfd h
  value:.(*lines)
  procwait h
  value}
 ,/collect'children}
peach:{[f;items]$[#items;parallel[f;items];()]}
