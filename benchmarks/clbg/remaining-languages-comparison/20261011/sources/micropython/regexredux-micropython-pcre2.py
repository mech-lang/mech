# Ordinary PCRE2 FFI only; pattern selection, counting and ordered passes in Python.
import sys
import ffi
import uctypes
import struct
lib = ffi.open("/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib")
compile = lib.func("p", "pcre2_compile_8", "sLippp")
jit = lib.func("i", "pcre2_jit_compile_8", "pi")
new_match = lib.func("p", "pcre2_match_data_create_from_pattern_8", "pp")
match = lib.func("i", "pcre2_match_8", "psLLipp")
vector = lib.func("p", "pcre2_get_ovector_pointer_8", "p")
substitute = lib.func("i", "pcre2_substitute_8", "psLLippsLpp")
free_code = lib.func("v", "pcre2_code_free_8", "p")
free_match = lib.func("v", "pcre2_match_data_free_8", "p")
class Regex:
    def __init__(self, pattern):
        error = bytearray(4)
        offset = bytearray(8)
        self.code = compile(pattern, len(pattern), 0, error, offset, 0)
        if not self.code:
            raise ValueError("PCRE2 compile error")

        self.data = new_match(self.code, 0)
    def count(self, text):
        offset = 0
        total = 0
        while match(self.code, text, len(text), offset, 0, self.data, 0) >= 0:
            start, end = struct.unpack("QQ", uctypes.bytearray_at(vector(self.data),16))
            total += 1
            offset = end if end > offset else offset+1
            if offset > len(text):
                break
        return total
    def replace(self, text, replacement):
        capacity = 2*len(text)+64
        while True:
            result = bytearray(capacity)
            size = bytearray(struct.pack("Q",capacity))
            status = substitute(self.code,text,len(text),0,0x1100,self.data,0,replacement,len(replacement),result,size)
            required = struct.unpack("Q",size)[0]
            if status == -48 and required > capacity:
                capacity = required
                continue
            if status < 0:
                raise ValueError("PCRE2 replacement error "+str(status))
            return bytes(result[:required])
    def close(self):
        free_match(self.data)
        free_code(self.code)
def replace(pattern, text, replacement):
    regex = Regex(pattern)
    result = regex.replace(text,replacement)
    regex.close()
    return result
source = sys.stdin.buffer.read()
text = replace(">.*\n|\n",source,b"")
patterns = ["agggtaaa|tttaccct","[cgt]gggtaaa|tttaccc[acg]","a[act]ggtaaa|tttacc[agt]t","ag[act]gtaaa|tttac[agt]ct","agg[act]taaa|ttta[agt]cct","aggg[acg]aaa|ttt[cgt]ccct","agggt[cgt]aa|tt[acg]accct","agggta[cgt]a|t[acg]taccct","agggtaa[cgt]|[acg]ttaccct"]
for pattern in patterns:
    regex = Regex(pattern)
    print(pattern,regex.count(text))
    regex.close()
result = text
for pattern, replacement in [("tHa[Nt]",b"<4>"),("aND|caN|Ha[DS]|WaS",b"<3>"),("a[NSt]|BY",b"<2>"),("<[^>]*>",b"|"),("\\|[^|][^|]*\\|",b"-")]:
    result = replace(pattern,result,replacement)
print("\n%d\n%d\n%d" % (len(source),len(text),len(result)))
