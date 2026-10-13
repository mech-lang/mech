// LP64 generic integer ABI declarations; no benchmark recurrence or executor.
module clbg::gmp;

struct Mpz
{
    int allocation;
    int size;
    ulong* limbs;
}

extern fn void init(Mpz* value) @cname("__gmpz_init");
extern fn void clear(Mpz* value) @cname("__gmpz_clear");
extern fn void set_ui(Mpz* value, ulong input) @cname("__gmpz_set_ui");
extern fn void add(Mpz* destination, Mpz* left, Mpz* right) @cname("__gmpz_add");
extern fn void sub(Mpz* destination, Mpz* left, Mpz* right) @cname("__gmpz_sub");
extern fn void mul_ui(Mpz* destination, Mpz* value, ulong factor) @cname("__gmpz_mul_ui");
extern fn void addmul_ui(Mpz* destination, Mpz* value, ulong factor) @cname("__gmpz_addmul_ui");
extern fn void divide(Mpz* destination, Mpz* numerator, Mpz* denominator) @cname("__gmpz_tdiv_q");
extern fn int compare(Mpz* left, Mpz* right) @cname("__gmpz_cmp");
extern fn ulong get_ui(Mpz* value) @cname("__gmpz_get_ui");
extern fn usz size_in_base(Mpz* value, int base) @cname("__gmpz_sizeinbase");
extern fn char* get_string(char* buffer, int base, Mpz* value) @cname("__gmpz_get_str");
extern char* version @cname("__gmp_version");
