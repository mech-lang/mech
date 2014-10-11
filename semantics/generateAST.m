function AST = generateAST(parse_tree)

    AST = parse_tree;
    
    disp(AST.tostring);
    
    % Remove unnecessary terminals and nodes
    AST = pruneTree(AST,'(');
    AST = pruneTree(AST,')');
    AST = pruneTree(AST,',');
    AST = pruneTree(AST,'e');
    AST = pruneTree(AST,'%');
    AST = pruneTree(AST,'Cmmt');
    AST = pruneTree(AST,'Empty');
    AST = pruneTree(AST,'EmpCmmt');
%     AST = pruneTree(AST,'ParenSfx');
%     AST = pruneTree(AST,'ISfx');
%     AST = pruneTree(AST,'IPfx');
    AST = pruneTree(AST,'Ident');
%     AST = pruneTree(AST,'CSfx');
%     AST = pruneTree(AST,'CPfx');
    AST = pruneTree(AST,'NConst');
    AST = pruneTree(AST,'Const');
%     AST = pruneTree(AST,'ArgLstSfx');
%     AST = pruneTree(AST,'ArgLstPfx');
%     AST = pruneTree(AST,'ExpSfx');
%     AST = pruneTree(AST,'ExpLst');
    AST = pruneTree(AST,'Exp');
%     AST = pruneTree(AST,'ParenPfx');
    AST = pruneTree(AST,'InfxOp');

    
    AST = organizeTree(AST);
    
end