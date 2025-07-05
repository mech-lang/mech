function [token_vals_endln,token_src_endln] = insertEndlines(token_vals,token_src)
    
    token_vals_endln = cell(1,length(token_vals)*2);
    token_vals_endln(1:2:end) = token_vals;
    token_vals_endln(2:2:end) = repmat({';'},1,length(token_vals));
    
    token_src_endln = cell(1,length(token_src)*2);
    token_src_endln(1:2:end) = token_src;
    token_src_endln(2:2:end) = repmat({';'},1,length(token_src));
    
    
end