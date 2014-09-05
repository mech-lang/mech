function token_array = fillTokenArray(token_array,tokens,locs)

    for i = 1:length(token_array)
        
        this_token = token_array{i};
        this_token(locs{i}) = tokens{i};
        token_array{i} = this_token;
        
    end

end