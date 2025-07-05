function tokens = formTokens(match_strings,token_rep)

     for i=1:length(match_strings)

         tokens{i} = {match_strings{i}{1}, token_rep};

     end

end