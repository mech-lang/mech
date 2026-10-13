:- module(clbg_observer,
          [temporary_begin/2,temporary_checked/4,stretch_checked/3,retain/2,retained_checked/3,
           map_begin/2,map_completed/4,maps_returned/1,pi_begin/1,pi_candidate/9,pi_finished/1]).
:- use_module(library(http/json), [json_write_dict/2]).
:- use_module(library(hashtable), [ht_pairs/2,ht_size/2]).

trace_stream(Stream) :-
    (   nb_current(clbg_trace_stream, Stream)
    ->  true
    ;   getenv('MECH_PROLOG_TRACE', Path),
        (exists_file(Path) -> throw(error(trace_already_exists, trace_stream/1)) ; true),
        open(Path, write, Stream, [encoding(utf8)]),
        nb_linkval(clbg_trace_stream, Stream)
    ).

emit(Event) :-
    thread_self(Worker),
    (nb_current(clbg_trace_ordinal, Previous) -> true ; Previous = 0),
    Ordinal is Previous + 1,
    nb_setval(clbg_trace_ordinal, Ordinal),
    put_dict(_{ordinal:Ordinal,worker:Worker}, Event, Actual),
    trace_stream(Stream),
    json_write_dict(Stream, Actual, [width(0)]),
    nl(Stream),
    flush_output(Stream).

require(Goal, Where) :-
    (call(Goal) -> true ; throw(error(observer_condition_failed(Where), clbg_observer))).

same_node(Node, [seen(Id,Other)|_], Id) :- same_term(Node, Other), !.
same_node(Node, [_|Rest], Id) :- same_node(Node, Rest, Id).

walk(Node, Seen0, Seen, Next0, Next, Id, Records) :-
    (   same_node(Node, Seen0, Existing)
    ->  Id = Existing, Seen = Seen0, Next = Next0, Records = []
    ;   require(compound_name_arity(Node, node, 2), actual_node_shape),
        Id = Next0,
        Following is Next0 + 1,
        Seen1 = [seen(Id,Node)|Seen0],
        arg(1, Node, Left),
        arg(2, Node, Right),
        (   Left == none
        ->  require(Right == none, both_leaf_children),
            LeftId = 0, RightId = 0, Leaf = true,
            Seen = Seen1, Next = Following, Children = []
        ;   walk(Left, Seen1, Seen2, Following, AfterLeft, LeftId, LeftRows),
            walk(Right, Seen2, Seen, AfterLeft, Next, RightId, RightRows),
            append(LeftRows, RightRows, Children),
            Leaf = false
        ),
        Records = [nodeinfo{id:Id,left:LeftId,right:RightId,leaf:Leaf}|Children]
    ).

snapshot(Tree, Graph, Seen) :-
    walk(Tree, [], Seen, 1, Next, Root, Nodes),
    Count is Next - 1,
    Graph = graph{root:Root,nodes:Nodes,distinct_nodes:Count}.

no_shared_node([], _).
no_shared_node([seen(_,Node)|Rest], Other) :-
    require(\+ same_node(Node, Other, _), concurrent_graph_disjointness),
    no_shared_node(Rest, Other).

retained_nodes(Nodes) :-
    (nb_current(clbg_retained, retained(_,_,Nodes,_)) -> true ; Nodes = []).

temporary_begin(Remaining, Depth) :-
    require(\+ nb_current(clbg_active_iteration, _), nested_iteration),
    thread_self(Worker),
    nb_linkval(clbg_active_iteration, iteration(Remaining,Depth,Worker)),
    emit(_{event:temporary_begin,remaining:Remaining,depth:Depth}).

temporary_checked(Remaining, Depth, Tree, Check) :-
    thread_self(Worker),
    require(nb_current(clbg_active_iteration, iteration(Remaining,Depth,Worker)), actual_iteration_owner),
    snapshot(Tree, Graph, Nodes),
    retained_nodes(Retained),
    no_shared_node(Nodes, Retained),
    emit(_{event:temporary_checked,remaining:Remaining,depth:Depth,check:Check,
           graph:Graph,concurrent_retained_disjoint:true}),
    nb_delete(clbg_active_iteration).

stretch_checked(Depth, Tree, Check) :-
    require(\+ nb_current(clbg_retained, _), stretch_precedes_retained),
    snapshot(Tree, Graph, _),
    emit(_{event:stretch_checked,depth:Depth,check:Check,graph:Graph,
           retained_across_callback:false}).

retain(Depth, Tree) :-
    require(\+ nb_current(clbg_retained, _), exactly_one_retained_root),
    snapshot(Tree, Graph, Nodes),
    nb_linkval(clbg_retained, retained(Depth,Tree,Nodes,Graph)),
    emit(_{event:retain,depth:Depth,graph:Graph}).

identity_matches([], _).
identity_matches([seen(Id,Node)|Rest], Current) :-
    require(same_node(Node, Current, Id), exact_retained_node_identity),
    identity_matches(Rest, Current).

retained_checked(Depth, Tree, Check) :-
    require(nb_current(clbg_retained, retained(Depth,Original,BeforeNodes,Before)), retained_root_exists),
    require(same_term(Tree, Original), retained_root_identity),
    snapshot(Tree, After, AfterNodes),
    require(Before == After, retained_graph_edges_unchanged),
    require(same_length(BeforeNodes,AfterNodes), retained_node_inventory_unchanged),
    identity_matches(BeforeNodes, AfterNodes),
    identity_matches(AfterNodes, BeforeNodes),
    emit(_{event:retained_checked,depth:Depth,check:Check,graph:After,
           actual_root_identity:true,all_actual_node_identities:true}),
    nb_delete(clbg_retained).

map_roots(Roots) :- (nb_current(clbg_map_roots, Roots) -> true ; Roots = []).

same_map(Table, [map(Id,Width,Other)|_], Id, Width) :- same_term(Table, Other), !.
same_map(Table, [_|Rest], Id, Width) :- same_map(Table, Rest, Id, Width).

map_begin(Width, Table) :-
    map_roots(Before),
    require(\+ same_map(Table, Before, _, _), distinct_map_root),
    length(Before, Count),
    Id is Count + 1,
    ht_size(Table, Size),
    arg(2, Table, Capacity),
    arg(3, Table, Buckets),
    compound_name_arity(Buckets, [], Arity),
    require((Size =:= 0, Capacity =:= 0, Arity =:= 1), actual_default_small_table),
    nb_linkval(clbg_map_roots, [map(Id,Width,Table)|Before]),
    emit(_{event:map_begin,map_id:Id,width:Width,size:Size,capacity:Capacity,bucket_arity:Arity}).

pair_rows([], []).
pair_rows([Key-Value|Pairs], [_{key:Key,value:Value}|Rows]) :- pair_rows(Pairs, Rows).

map_completed(Width, Table, Sequence, Length) :-
    map_roots(Roots),
    require(same_map(Table, Roots, Id, Width), actual_complete_map_root),
    ht_pairs(Table, Pairs),
    pair_rows(Pairs, Entries),
    ht_size(Table, Size),
    arg(2, Table, Capacity),
    arg(3, Table, Buckets),
    compound_name_arity(Buckets, [], Arity),
    emit(_{event:map_completed,map_id:Id,width:Width,sequence:Sequence,sequence_length:Length,
           size:Size,capacity:Capacity,bucket_arity:Arity,entries:Entries,
           actual_begin_root_identity:true,per_window_trace_recorded:false}).

returned_map_ids([], _, []).
returned_map_ids([Table|Tables], Roots, [Id|Ids]) :-
    require(same_map(Table, Roots, Id, _), original_returned_map_root),
    returned_map_ids(Tables, Roots, Ids).

maps_returned(Tables) :-
    map_roots(Roots),
    returned_map_ids(Tables, Roots, Ids),
    require(Ids == [1,2,3,4,5,6,7], actual_seven_map_order),
    emit(_{event:maps_returned,map_ids:Ids,actual_all_root_identities:true,
           single_worker:true,joins_required:false}),
    nb_delete(clbg_map_roots).

pi_begin(Requested) :-
    nb_setval(clbg_pi_requested, Requested),
    nb_setval(clbg_pi_observed_count, 0),
    emit(_{event:pi_begin,requested:Requested}).

pi_candidate(Count,Q,R,T,K,Numerator3,Numerator4,Digit3,Digit4) :-
    nb_getval(clbg_pi_requested, Requested),
    nb_getval(clbg_pi_observed_count, Previous),
    require(Previous =:= Count, actual_pi_emission_count),
    (   Digit3 =:= Digit4
    ->  Accepted = true, Next is Count + 1
    ;   Accepted = false, Next = Count
    ),
    nb_setval(clbg_pi_observed_count, Next),
    emit(_{event:pi_candidate,requested:Requested,count:Count,q:Q,r:R,t:T,k:K,
           numerator3:Numerator3,numerator4:Numerator4,quotient3:Digit3,quotient4:Digit4,
           accepted:Accepted}).

pi_finished(Requested) :-
    nb_getval(clbg_pi_observed_count, Actual),
    require(Actual =:= Requested, completed_pi_count),
    emit(_{event:pi_finished,requested:Requested,observed_accepted:Actual}),
    nb_delete(clbg_pi_requested),
    nb_delete(clbg_pi_observed_count).
