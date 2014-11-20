function t = makeNode(op,varargin)
    
    n = nargin - 1;
    
    t = tree(op);
    
    for i = 1:n
        t = t.graft(1,varargin{i});
    end
    
end