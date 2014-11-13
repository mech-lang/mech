function celldisp2(c)

    [n,m] = size(c);

    for i = 1:n
       
        for j = 1:m
            
            cell_contents = c{i,j};

            switch class(cell_contents);
                case 'double'
                    fprintf('%3.0f',cell_contents);
                    fprintf(repmat(' ',1,8-3*length(cell_contents)));
                case 'char'
                    fprintf('%10s   ',cell_contents);
                otherwise
                    keyboard
            end

        end

        fprintf('\n');

    end

    

end