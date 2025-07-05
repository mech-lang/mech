function empty_flag = isemptytree(t)

    for i = 1:length(t)
        empty_flag(i) = isempty(t(i).Node{1});
    end
    
end

