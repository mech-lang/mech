// Generic PCRE2 8-bit ABI declarations, no benchmark match/replacement kernel.
module clbg::pcre2;

extern fn void* compile(char* pattern, usz length, uint options, int* error, usz* error_offset, void* context) @cname("pcre2_compile_8");
extern fn void* match_data(void* pattern, void* context) @cname("pcre2_match_data_create_from_pattern_8");
extern fn int match(void* pattern, char* subject, usz length, usz start, uint options, void* result, void* context) @cname("pcre2_match_8");
extern fn usz* offsets(void* result) @cname("pcre2_get_ovector_pointer_8");
extern fn void free_pattern(void* pattern) @cname("pcre2_code_free_8");
extern fn void free_match_data(void* result) @cname("pcre2_match_data_free_8");
extern fn int config(uint what, void* result) @cname("pcre2_config_8");
