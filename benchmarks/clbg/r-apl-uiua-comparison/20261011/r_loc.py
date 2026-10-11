"""R LOC: noncomment lines, expanding packed independent statements/bindings.

Named call arguments are not bindings. Long literals and nested calls count as
one expression, consistently with the common comparison policy.
"""
import re

def clean(source):
 out=[]
 quote=None
 escaped=False
 comment=False
 for c in source:
  if comment:
   if c=='\n':comment=False;out.append(c)
   continue
  if quote:
   if c=='\n':out.append(c)
   if escaped:escaped=False
   elif c=='\\':escaped=True
   elif c==quote:quote=None
   continue
  if c=='#':comment=True
  elif c in ('"',"'",'`'):quote=c;out.append('S')
  else:out.append(c)
 return ''.join(out)

def normalize(source):
 cleaned=clean(source)
 physical=sum(bool(line.strip()) for line in cleaned.splitlines())
 expansions=[]
 parens=0
 for number,line in enumerate(cleaned.splitlines(),1):
  if not line.strip():continue
  count=max(1,line.count(';')+int(bool(line.rsplit(';',1)[-1].strip('{} \t\r\n')))) if ';' in line else 1
  arrows=len(re.findall(r'<<-|<-|->>|->',line))
  equals=0
  for i,c in enumerate(line):
   if c=='(':parens+=1
   elif c==')':parens-=1
   elif c=='=' and parens==0 and (i==0 or line[i-1] not in '=!<>') and (i+1==len(line) or line[i+1]!='='):equals+=1
  count=max(count,arrows+equals)
  if count>1:expansions.append({'line':number,'extra_lines':count-1,'reason':'Independent statements or assigned bindings packed on one source line.'})
 return {'physical_loc':physical,'normalized_loc':physical+sum(x['extra_lines'] for x in expansions),'expansions':expansions}

if __name__=='__main__':
 assert normalize('x <- 1; y <- 2')['normalized_loc']==2
 assert normalize('x <- y <- 1')['normalized_loc']==2
 assert normalize('x=1; y=2;')['normalized_loc']==2
 assert normalize('x <- c(a=1,b=2)')['normalized_loc']==1
 assert normalize('for (i in seq_len(n)) x[i] <- i')['normalized_loc']==1
 assert normalize('x <- "a;b" # ; comment')['normalized_loc']==1
 assert normalize('cat("x\\n"); cat("y\\n")')['normalized_loc']==2
 print('R LOC fixtures passed')
