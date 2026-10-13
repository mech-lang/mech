-module(clbg_process).
-export([start/1, join/1]).
start(Work) ->
    Owner = self(),
    Tag = make_ref(),
    {Pid, Monitor} = spawn_monitor(fun() -> Owner ! {Tag, self(), Work()} end),
    {Pid, Monitor, Tag}.
join({Pid, Monitor, Tag}) ->
    receive
        {Tag, Pid, Value} ->
            receive {'DOWN', Monitor, process, Pid, normal} -> Value;
                    {'DOWN', Monitor, process, Pid, Reason} -> erlang:error({worker_exit, Reason})
            end;
        {'DOWN', Monitor, process, Pid, Reason} -> erlang:error({worker_exit_without_result, Reason})
    end.
