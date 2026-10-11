# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by Ziong

const revcompdata = Dict(
   'A'=> 'T', 'a'=> 'T',
   'C'=> 'G', 'c'=> 'G',
   'G'=> 'C', 'g'=> 'C',
   'T'=> 'A', 't'=> 'A',
   'U'=> 'A', 'u'=> 'A',
   'M'=> 'K', 'm'=> 'K',
   'R'=> 'Y', 'r'=> 'Y',
   'W'=> 'W', 'w'=> 'W',
   'S'=> 'S', 's'=> 'S',
   'Y'=> 'R', 'y'=> 'R',
   'K'=> 'M', 'k'=> 'M',
   'V'=> 'B', 'v'=> 'B',
   'H'=> 'D', 'h'=> 'D',
   'D'=> 'H', 'd'=> 'H',
   'B'=> 'V', 'b'=> 'V',
   'N'=> 'N', 'n'=> 'N',
)


function print_buff(buff)
   out = String(take!(buff))
   l = length(out)
   n = 1
   outbuff = IOBuffer()
   for i in l:-1:1
      write(outbuff, revcompdata[out[i]])
      n % 60 == 0 ?  write(outbuff, '\n') : false
      n += 1
   end
   n % 60 > 1 ? write(outbuff, '\n') : false
   print(String(take!(outbuff)))
end

function main()
   buff = IOBuffer()
   line = readline()
   while !isempty(line)
      if line[1] == '>'
         buff.size > 0 ? print_buff(buff) : false
         println(line)
         buff = IOBuffer()
      else
         write(buff, line)
      end
      line = readline()
   end
   print_buff(buff)
end

main()
