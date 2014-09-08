%--------------------------------------------------------------------------
% Authors: Corey Montella
% Date Created:  09/08/2014
% Last Modified: 09/08/2014
% 
% Description: 
%  
% Parses tokens according to the synatax of the mech language
%
% INPUT:
%   
%   tokens = [token_val token]
%
% OUTPUT:
%
%   good - a binary flag indicating whether the input parsed or not
%   ast  - abstract syntax tree
%
% Changelog:
% 
% 09/08/2014 - CIM - Created
%--------------------------------------------------------------------------

function [good,ast] = parser(tokens)

    [good,tokens,subast] = program(tokens);

    ast = tree({'program' 'program'});
    ast = ast.graft(1,subast);
    ast = ast.removenode(2);

    disp('============================================================');
    %fprintf('(%d/%d) ',sum(good_lines==1),n);
    if good
        disp('PASSED! :D');
    else
        disp('FAILED :(');
    end
    disp('============================================================');

    % Clean ast of empty trees
    inds = isemptytree(ast);
    ast = ast(~inds);
    
end

% A program consists of at least one expression
function [good,tokens,ast] = program(tokens)
    
    [good, tokens, ast] = expression(tokens);
       
end

% An expression can be a variety of things, followed bt an optional
% expression.
function [good,tokens,ast] = expression(tokens)

    good = 1;
    
    % parse until it breaks or we run out of tokens
    i = 1;
    
    while good && ~isempty(tokens)
        
        [good,tokens,subast(i)] = orCom(tokens,     ...
                                  @definition,      ...
                                  @mathExpression,  ...
                                  @logicExpression, ...
                                  @listExpression,  ...
                                  @functionDefinition);
        
        % Exprssions are terminated by an end line delimeter
        tokens = tokenExpect(tokens,';');
                              
        next = getNextToken(tokens);
        
        % If we reach the end of a block, we reach the end of the
        % expression, so break out of the while loop
        if strcmp(next{1},'end')
            break;
        end
        
        i = i + 1;
        
    end
    
    % Form the ast based on the parsed lines
    ast = tree('exp');
    for j = 1:length(subast)
        ast = ast.graft(1,subast(j));
    end
    
end


function [good,tokens,ast] = functionDefinition(tokens)

    ast = tree('');

    % Match the function keyword and a function identifier
    [good,tokens,ast1] = andCom(tokens,                         ...
                               @(x)tokenTest(x,'_','function'), ...
                               @(x)tokenTest(x,'F'));
    if ~good
        return
    end
     
    ast = ast1(2);
    
    % Now parse the input argument expression
    [good,tokens,ast2] = inputArgs(tokens);
    
    if ~good
        return
    end
   
    % Test for the correct case
    [good,tokens,ast3] = orCom(tokens,         ...
                               @functionWhere, ...
                               @functionMath);
      
    ast = ast.graft(1,ast2);
    ast = ast.graft(1,ast3(2));    
    ast = ast.removenode(3);
                           
end

function [good,tokens,ast] = functionMath(tokens)

    % single line function definition
    [good,tokens,subast] = andCom(tokens,               ...
                               @(x)tokenTest(x,'='), ...
                               @mathExpression);
                           
    if good                    
        ast(1) = subast(1);
        ast(2) = tree('functionMath');
        ast(2) = ast(2).graft(1,subast(2));
    else
        ast = tree();
    end                     
                           
end


function [good,tokens,ast] = functionWhere(tokens)

    % output args, followed by where clause
    [good,tokens,subast] = andCom(tokens,            ...
                               @(x)tokenTest(x,'='), ...
                               @outputArgs,          ...
                               @endline,             ...
                               @whereClause);
    if good                    
        ast(1) = subast(1);
        ast(2) = tree('functionWhere');
        ast(2) = ast(2).graft(1,subast(2));
        ast(2) = ast(2).graft(1,subast(4));
    else
        ast = tree();
    end
                              
end

function [good,tokens,ast] = outputArgs(tokens)

    % Output list can either be a single identifier or an enclosed list of 
    % identifiers
    
    multiple_args = @(x)andCom(x,                    ...
                               @(x)tokenTest(x,'['), ...
                               @argList,             ...
                               @(x)tokenTest(x,']'));
                           
                           
    [good,tokens,ast] = orCom(tokens,        ...
                              multiple_args, ...
                              @identifier);
    if length(ast) > 1
        ast = ast(2);
        ast.Node{1}(1)={'outarglist'};
    end
                          
                          
end

function [good,tokens,ast] = whereClause(tokens)


    [good,tokens,subast] = andCom(tokens,                       ...
                                  @(x)tokenTest(x,'_','where'), ...
                                  @endline,                     ...
                                  @expression,                  ...
                                  @endBlock);
               
    if good                         
        ast = subast(1);
        ast = ast.graft(1,subast(3));
        ast = ast.removenode(2);
    else
        ast = tree();
    end

end

function [good,tokens,ast] = inputArgs(tokens)

    [good,tokens,subast] = andCom(tokens,               ...
                               @(x)tokenTest(x,'('), ...
                               @argList,             ...
                               @(x)tokenTest(x,')'));  
     if good 
         ast = subast(2);
     else
         ast = tree();
     end
                           
end

function [good,tokens,ast] = argList(tokens)

    good = 1;
    
    ast = tree({'arglist','arglist'});
    while good
        
        next = getNextToken(tokens);
        
        if next{2} == '$'
            tokens = consume(tokens);
            ast = ast.addnode(1,next);
        elseif next{2} == ','
            tokens = consume(tokens);
        elseif next{2} == ')' || next{2} == ']'
            break;
        else
            good = 0;
            return;
        end
    end
    
end

function [good,tokens,ast] = definition(tokens)

    definition_suffix = @(x)orCom(x,                ...
                                  @mechExpression,  ...
                                  @logicExpression, ...
                                  @mathExpression,  ...
                                  @listExpression,  ...
                                  @string);

    [good,tokens,subast] = andCom(tokens,      ...
                                  @identifier, ...
                                  @assignment, ...
                                  definition_suffix);
                              
    if good
        ast = subast(1);
        ast = ast.graft(1,subast(3));
    else
        ast = tree();
    end
              
end

function [good,tokens,ast] = logicExpression(tokens)
     
    [good,tokens,subast] = andCom(tokens,          ...
                               @mathExpression, ...
                               @logicOperator,  ...
                               @mathExpression);
    if good                        
        ast = subast(2);
        ast = ast.graft(1,subast(1));
        ast = ast.graft(1,subast(3));
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
        ast = tree({'[]','['});
        ast = ast.graft(1,subast(2));
        delnode = length(ast.Node) + 1;
        ast = ast.graft(1,subast(3));
        ast = ast.removenode(delnode);        
    else
        ast = tree();
    end
                           
end

function [good,tokens,ast] = listLoop(tokens)

    good = 1;

    next = getNextToken(tokens);
    test = ',#.(@';
    
    i = 1;
    while good && ~isempty(intersect(next{2},test))
        
        if next{2} == ','
            tokens = consume(tokens);
        end
        
        [good,tokens,subast(i)] = mathExpression(tokens);
        i = i + 1;
        next = getNextToken(tokens);
        
    end
    
    % Build a sub tree
    ast = tree({'[]','[]'});
    if i > 1
        for j = 1:length(subast);
            ast = ast.graft(1,subast(j));
        end
    end
    
end

function [good,tokens,ast] = mathExpression(tokens)

   [good,tokens,ast] = mathParser(tokens);

end

%% Terminals

function [good,tokens,ast] = mechOperator(tokens)
    
    [good,tokens,ast] = andCom(tokens, ...
                               @(x)tokenTest(x,'~'));
        
end

function [good,tokens,ast] = logicOperator(tokens)
    
    [good,tokens,ast] = andCom(tokens, ...
                               @(x)tokenTest(x,'>'));
        
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

function [good,tokens,ast] = endline(tokens)
    
    [good,tokens] = andCom(tokens, ...
                           @(x)tokenTest(x,';'));
                       
	ast = tree();
        
end

function [good,tokens,ast] = endBlock(tokens)
    
    [good,tokens] = andCom(tokens, ...
                           @(x)tokenTest(x,'_','end'));
                       
	ast = tree();
        
end