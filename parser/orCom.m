function [good,tokens,ast] = orCom(tokens,varargin)

    n = nargin - 1;

    for i = 1:n
        
        fun = varargin{i};

        [good,tokens1,ast] = fun(tokens);
        
        if good
            tokens = tokens1;
            return;
        end 
        
    end
    
end