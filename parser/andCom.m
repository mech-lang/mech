function [good,tokens,ast] = andCom(tokens,varargin)
    
    n = nargin - 1;
    
    for i = 1:n
        
        fun = varargin{i};

        [good,tokens,ast(i)] = fun(tokens);

        if ~good
            return;
        end 
        
    end
       
end