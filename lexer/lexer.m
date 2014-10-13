%--------------------------------------------------------------------------
% Authors: Corey Montella
% Date Created:  09/05/2014
% Last Modified: 10/13/2014
% 
% Description: 
%  
% Tokenizes a source file for the mech language.
%
% INPUT:
%
%   src - mech source file. This is a cell array where each cell is a line
%   of source.
%
% OUTPUT:
%   
%   tokens - tokenized source cell array. Contains as many cells as the
%   source code has lines. Each line is broken down into tokens.
%
%   token_src - semantics of the tokenized source code, as a cell array.
%   Each cell corresponds to a line in the source. The symbols denote a
%   type, which is defined in lexer.m for now.
%
% Changelog:
%
% 10/13/2014 - CIM - Changed rules for lexing brackets. Now each bracket
%                    class has its own token.
%                  - Mechanized token replacement
% 10/12/2014 - CIM - Added new token for function identifiers
%                  - Changed token for floats from '@' to '&'
%                  - Added new rule to lex a urnary operator at the start
%                    of a line.
% 09/22/2014 - CIM - Remove blank lines before lex
% 09/05/2014 - CIM - Created
%--------------------------------------------------------------------------

function tokens = lexer(src)

    %% Regular Expressions
    variable = '([a-z]|[A-Z])([A-Za-z_0-9]+)?';
    string_const = '".+"';
    digit_excluding_zero = '[1-9]';
    digit = ['(0|' digit_excluding_zero ')'];
    natural_number = ['(' digit_excluding_zero digit '+' '|' digit_excluding_zero ')' ];
    integer = ['(0|' natural_number ')'];
    float = ['(|0|' natural_number ')\.[0-9]+'];
    plus_minus_operator = '+|-';
    times_divide_operator = '*|/';
    exponent_operator = '\^';
    definition = '=';
    mech_op = '~>|<~|::>';
    o1_bracket = '(';
    o2_bracket = '[';
    o3_bracket = '{';
    c1_bracket = ')';
    c2_bracket = ']';
    c3_bracket = '}';
    endline = ';';
    separator = ',';
    comment = '%.+';
    prefix = '(?<=(+|*|\^|,|\(|=))+';
    fxn_identifier = '\$(?=(\())';
    fxn_def_identifier = '(?<=_)(@|\$)';
    keyword = '\<function|end|where\>';
    logic_op = '~=|==|>=|<=|>|<';
    
    % Token replacements
    keyword_rep         = '_';
    endline_rep         = ';';
    comment_rep         = ''; 
    logic_op_rep        = '>';
    mech_rep            = '~';
    plus_minus_op_rep   = '+';
    times_divide_op_rep = '*';
    exponent_op_rep     = '^';
    def_op_rep          = '=';
    var_rep             = '$';
    sep_rep             = ',';
    float_rep           = '.';
    int_rep             = '#';
    str_rep             = '"';
    o1_bracket_rep      = '(';
    o2_bracket_rep      = '[';
    o3_bracket_rep      = '{';
    c1_bracket_rep      = ')';
    c2_bracket_rep      = ']';
    c3_bracket_rep      = '}';
    prefix_rep          = '-';
    fxn_rep             = '@';
    fxn_def_rep         = 'F';

    rep = {endline_rep keyword_rep str_rep var_rep logic_op_rep mech_rep plus_minus_op_rep times_divide_op_rep exponent_op_rep ...
           def_op_rep o1_bracket_rep c1_bracket_rep o2_bracket_rep c2_bracket_rep, ...
           o3_bracket_rep c3_bracket_rep sep_rep float_rep int_rep};

    %% Tokenize
    %  Generates tokens from source on all lines at once.
    
    if isempty(src)
        tokens = {[],[]};
        return;
    end
    
    % Remove blank lines
    filled_cells = ~cellfun(@isempty,src);
    token_src = src(filled_cells);
    
    matches = {};
        
    % Match endlines
    [matches{end+1},token_src] = regexprep2(token_src,endline,endline_rep);
    
    % Match comments, which removes them from the source
    [~,token_src] = regexprep2(token_src,comment,comment_rep);
    
    % Match reserved keywords
    [matches{end+1},token_src] = regexprep2(token_src,keyword,keyword_rep);
    
    % Match string constants
    [matches{end+1},token_src] = regexprep2(token_src,string_const,str_rep);
    
    % Match identifiers
    [matches{end+1},token_src] = regexprep2(token_src,variable,var_rep);
    
    % Match logic operators
    [matches{end+1},token_src] = regexprep2(token_src,logic_op,logic_op_rep);
    
    % Match mech operators
    [matches{end+1},token_src] = regexprep2(token_src,mech_op,mech_rep);
    
    % Match operators
    [matches{end+1},token_src] = regexprep2(token_src,plus_minus_operator,plus_minus_op_rep);
    [matches{end+1},token_src] = regexprep2(token_src,times_divide_operator,times_divide_op_rep);
    [matches{end+1},token_src] = regexprep2(token_src,exponent_operator,exponent_op_rep);
    
    % Match definition declarations
    [matches{end+1},token_src] = regexprep2(token_src,definition,def_op_rep);

    % Match brackets
    % ()
    [matches{end+1},token_src] = regexprep2(token_src,o1_bracket,o1_bracket_rep);
    [matches{end+1},token_src] = regexprep2(token_src,c1_bracket,c1_bracket_rep);
    
    % []
    [matches{end+1},token_src] = regexprep2(token_src,o2_bracket,o2_bracket_rep);
    [matches{end+1},token_src] = regexprep2(token_src,c2_bracket,c2_bracket_rep);
    
    % {}
    [matches{end+1},token_src] = regexprep2(token_src,o3_bracket,o3_bracket_rep);
    [matches{end+1},token_src] = regexprep2(token_src,c3_bracket,c3_bracket_rep);

    % Match serparators
    [matches{end+1},token_src] = regexprep2(token_src,separator,sep_rep);
    
    % Match floats
    [matches{end+1},token_src] = regexprep2(token_src,float,float_rep);

    % Match integers
    [matches{end+1},token_src] = regexprep2(token_src,integer,int_rep);
       
    % Remove whitespace
    token_src = regexprep(token_src,'\s+','');
    
    %% Sort token values 
    %  Places the token values in the correct order as they appear in source
    
    token_vals = createTokenArray(token_src);
    
    for i = 1:length(matches)
        locs = strfind(token_src,rep{i});
        token_vals = fillTokenArray(token_vals,matches{i},locs);
    end
    
    %% Perform context-based matches
    
    % Match prefix operators
    [~,token_src] = regexprep2(token_src,prefix,prefix_rep);
    [~,token_src] = regexprep2(token_src,'^\+',prefix_rep);
    
    % Match function call identifiers
    [~,token_src] = regexprep2(token_src,fxn_identifier,fxn_rep);

    % Match function definition identifiers
    [~,token_src] = regexprep2(token_src,fxn_def_identifier,fxn_def_rep);
                
    % Delete empty lines
    empty_lines_mask = strcmp(token_src,'');
    token_src(empty_lines_mask) = [];
    token_vals(empty_lines_mask) = [];
    
    %% Form output
    [token_vals,token_src] = insertEndlines(token_vals,token_src);
    
    % Reshape tokens to a continuous stream
    token_tags = num2cell([token_src{:}]);
    token_vals = [token_vals{:}];
    
    % Combine values and tags into tuples
    tokens = cell(1,length(token_vals));
    for i = 1:length(token_tags)
        tokens{i} = [token_vals(i) token_tags(i)];
    end

end