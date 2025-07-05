%--------------------------------------------------------------------------
% Authors: Corey Montella
% Date Created:  09/08/2014
% Last Modified: 10/12/2014
% 
% Description: 
%  
% Parses tokens according to the synatax of the mech language
%
% INPUT:
%   
%   tokens -    
%
% OUTPUT:
%
%   good - a binary flag indicating whether the input parsed or not
%   ast  - abstract syntax tree
%
% Changelog:
% 
% 10/12/2014 - CIM - Overhauled
% 09/08/2014 - CIM - Created
%--------------------------------------------------------------------------

function [good, ast] = parser2(tokens)

    [good,ast] = program(tokens);

end

% Test if the tokens represent a well-formed program
function [good,tokens,ast] = program(tokens)
    disp('program...');
    
    n = size(tokens,1);
    
    good_lines = zeros(1,n);

    % Check each line in succession
    for i = 1:n
        
        fprintf('================================ Line %d ====================\n',i);

        current_line = tokens(i,:);
        disp(current_line{2})
        [good_lines(i), unused_tokens, ast(i)] = orCom(current_line,    ...
                                                    @emptyComment,   ...
                                                    @definition,     ...
                                                    @mechExpression, ...
                                                    @mathExpression, ...
                                                    @listExpression, ...
                                                    @string);
                                        
        % Optionally parse a comment
        next = getNextToken(unused_tokens);

        if next == '%'
            unused_tokens = consume(unused_tokens);
        end                       
                       
        if ~isempty(unused_tokens{2})
            good_lines(i) = 0;
        end
                                   
    end
    
    % If all the lines parsed, then the program matches the grammar
    good = all(good_lines);
    
    disp('============================================================');
    fprintf('(%d/%d) ',sum(good_lines==1),n);
    if good
        disp('PASSED! :D');
    else
        disp('FAILED :(');
    end
    disp('============================================================');
    
end

function [good,tokens,ast] = definition(tokens)

    definition_suffix = @(x)orCom(x, ...
                                  @mechExpression, ...
                                  @mathExpression, ...
                                  @listExpression, ...
                                  @string);

    [good,tokens,subast] = andCom(tokens, ...
                                  @identifier, ...
                                  @assignment, ...
                                  definition_suffix);
      
    if good
        ast = subast(1);
        ast = ast.graft(1,subast(2));
        ast = ast.graft(2,subast(3));
    else
        ast = tree();
    end
              
end

function [good,tokens,ast] = mechExpression(tokens)
     
    [good,tokens,ast] = andCom(tokens,          ...
                               @mathExpression, ...
                               @mechOperator,   ...
                               @mathExpression);
                   
                   
                   
end

function [good,tokens,ast] = listExpression(tokens)

    [good,tokens,subast] = andCom(tokens,               ...
                                  @(x)tokenTest(x,'['), ...
                                  @mathExpression,      ...
                                  @listLoop,            ...
                                  @(x)tokenTest(x,']'));
                           
    if good 
        ast = makeNode(subast(3),subast(2));
    else
        ast = tree();
    end
                           
end

function [good,tokens,ast] = listLoop(tokens)

    good = 1;

    next = getNextToken(tokens);
    test = ',#&(@';
    
    i = 1;
    while good && ~isempty(intersect(next,test))
        
        if next == ','
            tokens = consume(tokens);
        end
        
        [good,tokens,subast(i)] = mathExpression(tokens);
        i = i + 1;
        next = getNextToken(tokens);
        
    end

    % Build a sub tree
    ast = tree('[]');
    if i > 1
        for j = 1:length(subast)
            ast = ast.graft(1,subast(j));
        end
    end
    
end

function [good,tokens,ast] = mathExpression(tokens)

   [good,tokens,ast] = mathParser(tokens);
   
end

function [good,tokens,ast] = emptyComment(tokens)

    [good,tokens,ast] = orCom(tokens, ...
                              @empty, ...
                              @comment);
end

%% Terminals

function [good,tokens,ast] = mechOperator(tokens)
    
    [good,tokens,ast] = andCom(tokens, ...
                               @(x)tokenTest(x,'~'));
        
end

function [good,tokens,ast] = string(tokens)
    
    [good,tokens,ast] = andCom(tokens, ...
                               @(x)tokenTest(x,'"'));
        
end

function [good,tokens,ast] = assignment(tokens)
    
    [good,tokens,ast] = andCom(tokens, ...
                               @(x)tokenTest(x,'='));
        
end

function [good,tokens,ast] = identifier(tokens)
    
    [good,tokens,ast] = andCom(tokens, ...
                               @(x)tokenTest(x,'$'));
        
end

function [good,tokens,ast] = comment(tokens)
    
    [good,tokens] = andCom(tokens, ...
                               @(x)tokenTest(x,'%'));
   
    ast = tree('');
    
end

function [good,tokens,ast] = empty(tokens)
    
    [good,tokens] = andCom(tokens, ...
                           @(x)tokenTest(x,'e'));
                       
	ast = tree('');
        
end