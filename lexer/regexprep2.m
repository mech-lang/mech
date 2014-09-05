function [matches,src_rep] = regexprep2(src,exp,rep)

        matches = regexp(src,exp,'match');
        src_rep = regexprep(src,exp,rep); 

end