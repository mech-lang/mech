function acc = debugPrint(acc,text,tokens)

    acc.blanks = [acc.blanks '.'];

    fprintf(acc.blanks);
    fprintf([text genWhite(20-length(text)) genWhite(40-length(acc.blanks)) '%s \n'],tokens{2});
    
end