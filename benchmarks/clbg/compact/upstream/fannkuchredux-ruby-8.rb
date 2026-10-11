# The Computer Language Benchmarks Game
#   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
#   Naive transliteration from Rex Kerr's Scala program
#   contributed by Isaac Gouy 
#

def fannkuch(n)
   perm1 = Array.new(n) {|index| index}       
   perm = Array.new(perm1)
   count = Array.new(perm1)       
   f = flips = nperm = checksum = j = k = r = 0          
  
   r = n
   while r > 0 do 
      i = 0  
      while (r != 1) do count[r-1] = r; r -= 1 end  
      while (i < n) do perm[i] = perm1[i]; i += 1 end  
        
      # Count flips and update max and checksum
      f = 0
      k = perm[0]  
      while k != 0 do
         i = 0  
         while 2*i < k do         
            t = perm[i]; perm[i] = perm[k-i]; perm[k-i] = t  
            i += 1           
         end
         k = perm[0]
         f += 1   
      end
      if f > flips then flips = f end         
      if (nperm & 0x1) == 0 then checksum += f else checksum -= f end
   
      # Use incremental change to generate another permutation   
      more = true
      while more do  
         if r == n then  
            puts checksum           
            return flips 
         end
         p0 = perm1[0]
         i = 0
         while i < r do
            j = i + 1
            perm1[i] = perm1[j]
            i = j            
         end
         perm1[r] = p0 
         
         count[r] -= 1        
         if count[r] > 0 then more = false else r += 1 end
      end
      nperm += 1
   end   
   return flips      
end

n = (ARGV[0] || 7).to_i   
puts "Pfannkuchen(#{n}) = #{fannkuch(n)}"
