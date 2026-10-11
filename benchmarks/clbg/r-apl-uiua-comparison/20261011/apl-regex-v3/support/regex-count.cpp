// General nonoverlapping regex count through GNU APL's native interface.
// GNU APL 2.0 interface conventions; this file is GPL-3.0-or-later.
// PCRE2 repeated matching follows its documented pcre2_next_match API.
// No benchmark patterns, record parsing, substitutions, workload logic or I/O.
#include "Native_interface.hh"
#include "Value.hh"
#define PCRE2_CODE_UNIT_WIDTH 32
#include <pcre2.h>
#include <cstring>
#include <limits>
#include <memory>
#include <vector>

class NativeFunction;

static std::vector<PCRE2_UCHAR> characters(const Value_P &value) {
    if (value->get_rank() > 1) RANK_ERROR;
    if (!value->is_char_string()) DOMAIN_ERROR;
    std::vector<PCRE2_UCHAR> text;
    text.reserve(value->element_count() + 1);
    for (ShapeItem i = 0; i < value->element_count(); ++i)
        text.push_back(static_cast<PCRE2_UCHAR>(value->get_cravel(i).get_char_value()));
    text.push_back(0);
    return text;
}

static Fun_signature signature() { return SIG_Z_A_F2_B; }
static bool close_binding(Cause, const NativeFunction *) { return true; }

static Token count_matches(Value_P pattern_value, Value_P subject_value,
                           uint32_t compile_options, bool jit) {
    const auto pattern = characters(pattern_value);
    const auto subject = characters(subject_value);
    int error = 0;
    PCRE2_SIZE error_offset = 0;
    std::unique_ptr<pcre2_code, decltype(&pcre2_code_free)> code(
        pcre2_compile(pattern.data(), pattern.size() - 1, compile_options,
                      &error, &error_offset, nullptr), &pcre2_code_free);
    if (!code) {
        MORE_ERROR() << "PCRE2 pattern error " << error << " at code unit " << error_offset;
        DOMAIN_ERROR;
    }
    if (jit) pcre2_jit_compile(code.get(), PCRE2_JIT_COMPLETE);
    std::unique_ptr<pcre2_match_data, decltype(&pcre2_match_data_free)> matches(
        pcre2_match_data_create_from_pattern(code.get(), nullptr), &pcre2_match_data_free);
    if (!matches) DOMAIN_ERROR;
    PCRE2_SIZE start = 0;
    uint32_t options = 0;
    APL_Integer total = 0;
    for (;;) {
        const int result = pcre2_match(code.get(), subject.data(), subject.size() - 1,
                                      start, options, matches.get(), nullptr);
        if (result == PCRE2_ERROR_NOMATCH) break;
        if (result < 0) {
            MORE_ERROR() << "PCRE2 matching error " << result;
            DOMAIN_ERROR;
        }
        if (total == std::numeric_limits<APL_Integer>::max()) DOMAIN_ERROR;
        ++total;
        // This standard library helper supplies global-search progress,
        // including empty matches and Unicode boundaries. One reusable match
        // frame is retained; no capture strings or all-match arrays are made.
        if (!pcre2_next_match(matches.get(), &start, &options)) break;
    }
    return Token(TOK_APL_VALUE1, IntScalar(total, LOC));
}

static Token dyad(Value_P a, Value_P b, const NativeFunction *) {
    return count_matches(a, b, 0, false);
}

static Token axis_dyad(Value_P a, Value_P x, Value_P b, const NativeFunction *) {
    if (x->get_rank() > 1) RANK_ERROR;
    const ShapeItem length = x->element_count();
    if (length != 1 && length != 2) LENGTH_ERROR;
    const APL_Integer flags = length == 2 ? x->get_cravel(0).get_near_int() : 0;
    const APL_Integer jit = x->get_cravel(length - 1).get_near_int();
    if (flags < 0 || static_cast<uint64_t>(flags) > std::numeric_limits<uint32_t>::max()) DOMAIN_ERROR;
    if (jit != 0 && jit != 1) DOMAIN_ERROR;
    return count_matches(a, b, static_cast<uint32_t>(flags), jit == 1);
}

extern "C" void *get_function_mux(const char *name) {
    if (!std::strcmp(name, "get_signature")) return reinterpret_cast<void *>(&signature);
    if (!std::strcmp(name, "close_fun")) return reinterpret_cast<void *>(&close_binding);
    if (!std::strcmp(name, "eval_AB")) return reinterpret_cast<void *>(&dyad);
    if (!std::strcmp(name, "eval_AXB")) return reinterpret_cast<void *>(&axis_dyad);
    return nullptr;
}
