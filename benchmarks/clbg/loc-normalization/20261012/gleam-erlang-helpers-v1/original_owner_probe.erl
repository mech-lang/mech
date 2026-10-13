%% Original source scanner/parser evidence only; no benchmark kernel or LOC decision.
-module(original_owner_probe).
-export([main/1]).

main([Source]) ->
    {ok, Bytes} = file:read_file(Source),
    Value = case erl_scan:string(unicode:characters_to_list(Bytes), {1, 1}, [text, return_comments]) of
        {ok, Tokens, ScanEnd} ->
            Code = [T || T <- Tokens, erl_scan:category(T) =/= comment],
            case forms(Code, [], []) of
                {ok, Forms} -> #{status => <<"original-source-scan-and-parse-passed">>,
                    tokens => [token(T) || T <- Tokens], forms => term(Forms), scan_end => term(ScanEnd)};
                {error, ParseError} -> #{status => <<"original-source-parse-error">>, error => term(ParseError)}
            end;
        {error, ScanError, ErrorEnd} -> #{status => <<"original-source-scan-error">>, error => term(ScanError), scan_end => term(ErrorEnd)}
    end,
    [begin {module, Name} = code:ensure_loaded(Name) end || Name <- [erl_scan, erl_parse, erl_anno, json]],
    Modules = [#{module => atom_to_binary(Name), path => unicode:characters_to_binary(code:which(Name))}
        || Name <- [erl_scan, erl_parse, erl_anno, json]],
    io:put_chars(json:encode(Value#{actual_loaded_modules => Modules})),
    halt().

forms([], [], Acc) -> {ok, lists:reverse(Acc)};
forms([], Pending, _) -> {error, {unterminated_form, Pending}};
forms([Token | Rest], Pending, Acc) ->
    case erl_scan:category(Token) of
        dot ->
            case erl_parse:parse_form(lists:reverse([Token | Pending])) of
                {ok, Form} -> forms(Rest, [], [Form | Acc]);
                {error, Error} -> {error, Error}
            end;
        _ -> forms(Rest, [Token | Pending], Acc)
    end.

token(Token) ->
    {Line, Column} = erl_scan:location(Token),
    #{kind => atom_to_binary(erl_scan:category(Token)), line => Line, column => Column,
        text => unicode:characters_to_binary(erl_scan:text(Token)), value => term(erl_scan:symbol(Token))}.

term(Term) when is_tuple(Term) -> [term(Value) || Value <- tuple_to_list(Term)];
term(Term) when is_list(Term) -> [term(Value) || Value <- Term];
term(Term) when is_atom(Term) -> atom_to_binary(Term);
term(Term) when is_integer(Term); is_float(Term); is_binary(Term) -> Term.
