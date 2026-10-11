// General literal global regex substitution through GNU APL's native interface.
// Interface conventions: GNU APL 2.0 src/native/template.hh, GPL-3.0-or-later.
// This file is GPL-3.0-or-later. No benchmark patterns, sequence, I/O or counts.
#include "Native_interface.hh"
#include "Value.hh"
#define PCRE2_CODE_UNIT_WIDTH 32
#include <pcre2.h>
#include <cstring>
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
    // A nonnull address also supports empty strings with an explicit length 0.
    text.push_back(0);
    return text;
}

static Fun_signature signature() { return SIG_Z_A_F2_B; }
static bool close_binding(Cause, const NativeFunction *) { return true; }

static Token substitute(Value_P specification, Value_P subject_value, bool jit) {
    if (specification->get_rank() != 1) RANK_ERROR;
    if (specification->element_count() != 2) LENGTH_ERROR;
    Value_P pattern_value = specification->get_cravel(0).to_value(LOC);
    Value_P replacement_value = specification->get_cravel(1).to_value(LOC);
    const auto pattern = characters(pattern_value);
    const auto replacement = characters(replacement_value);
    const auto subject = characters(subject_value);
    int error = 0;
    PCRE2_SIZE error_offset = 0;
    std::unique_ptr<pcre2_code, decltype(&pcre2_code_free)> code(
        pcre2_compile(pattern.data(), pattern.size() - 1, 0,
                      &error, &error_offset, nullptr), &pcre2_code_free);
    if (!code) {
        MORE_ERROR() << "PCRE2 pattern error " << error << " at code unit " << error_offset;
        DOMAIN_ERROR;
    }
    // JIT is an ordinary optional PCRE2 acceleration; unsupported builds fall
    // back to the same matcher without changing substitution semantics.
    if (jit) pcre2_jit_compile(code.get(), PCRE2_JIT_COMPLETE);
    std::vector<PCRE2_UCHAR> output(subject.size());
    PCRE2_SIZE output_length = output.size();
    const uint32_t options = PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_LITERAL
                           | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH;
    int result = pcre2_substitute(code.get(), subject.data(), subject.size() - 1,
        0, options, nullptr, nullptr, replacement.data(), replacement.size() - 1,
        output.data(), &output_length);
    if (result == PCRE2_ERROR_NOMEMORY) {
        // The documented overflow option computes the exact required capacity.
        // No per-character masks, prefix arrays or match containers are kept.
        output.resize(output_length);
        output_length = output.size();
        result = pcre2_substitute(code.get(), subject.data(), subject.size() - 1,
            0, options, nullptr, nullptr, replacement.data(), replacement.size() - 1,
            output.data(), &output_length);
    }
    if (result < 0) {
        MORE_ERROR() << "PCRE2 substitution error " << result;
        DOMAIN_ERROR;
    }
    Value_P value(static_cast<ShapeItem>(output_length), LOC);
    value->set_proto_Spc();
    for (PCRE2_SIZE i = 0; i < output_length; ++i)
        value->next_ravel_Char(static_cast<Unicode>(output[i]));
    value->check_value(LOC);
    return Token(TOK_APL_VALUE1, value);
}

static Token dyad(Value_P a, Value_P b, const NativeFunction *) {
    return substitute(a, b, false);
}

static Token axis_dyad(Value_P a, Value_P x, Value_P b, const NativeFunction *) {
    if (x->element_count() != 1) LENGTH_ERROR;
    const APL_Integer mode = x->get_cfirst().get_near_int();
    if (mode != 0 && mode != 1) DOMAIN_ERROR;
    return substitute(a, b, mode == 1);
}

extern "C" void *get_function_mux(const char *name) {
    if (!std::strcmp(name, "get_signature")) return reinterpret_cast<void *>(&signature);
    if (!std::strcmp(name, "close_fun")) return reinterpret_cast<void *>(&close_binding);
    if (!std::strcmp(name, "eval_AB")) return reinterpret_cast<void *>(&dyad);
    if (!std::strcmp(name, "eval_AXB")) return reinterpret_cast<void *>(&axis_dyad);
    return nullptr;
}
