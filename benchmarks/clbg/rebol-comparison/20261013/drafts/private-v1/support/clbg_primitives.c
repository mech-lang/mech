/* Source-only general extension proposal for exact Oldes/Rebol3 3.22.1.
 * Build only against its complete generated and pinned SDK, never a partial
 * handwritten RL_LIB layout. No SDK generation/build/load has occurred here.
 * All algorithms and pattern/count/replacement orchestration remain Rebol.
 */
#include "reb-host.h"
#include "host-lib.h"
#include <errno.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <gmp.h>
#define PCRE2_CODE_UNIT_WIDTH 8
#include <pcre2.h>

RL_LIB *RL;

static const char module_source[] =
    "Rebol [Title: {General signed arithmetic, byte matching and IO} "
    "Name: mech-clbg-general-primitives Type: module "
    "Exports: [gmp-add gmp-subtract gmp-multiply gmp-floor-divide gmp-compare "
    "pcre-find read-stdin-chunk write-stdout format-fixed]]\n"
    "gmp-add: command [a [binary!] b [binary!]]\n"
    "gmp-subtract: command [a [binary!] b [binary!]]\n"
    "gmp-multiply: command [a [binary!] b [binary!]]\n"
    "gmp-floor-divide: command [a [binary!] b [binary!]]\n"
    "gmp-compare: command [a [binary!] b [binary!]]\n"
    "pcre-find: command [pattern [binary!] subject [binary!] start [integer!]]\n"
    "read-stdin-chunk: command []\n"
    "write-stdout: command [data [binary!]]\n"
    "format-fixed: command [value [decimal!] digits [integer!]]\n";

enum { CMD_ADD, CMD_SUBTRACT, CMD_MULTIPLY, CMD_FLOOR_DIVIDE, CMD_COMPARE,
       CMD_FIND, CMD_READ, CMD_WRITE, CMD_FORMAT };

/* Make_Series uses the signed MAX_I32 bound, not the RXIARG unsigned index
 * range. Leave room for its terminator/alignment rather than approaching it. */
#define BYTE_LIMIT ((size_t)INT32_MAX - 64U)
#define IO_CHUNK 32768U

static int fail(RXIFRM *frame, const char *message) {
    RXA_SERIES(frame, 1) = (void *)message;
    return RXR_ERROR;
}

static int bytes_view(RXIFRM *frame, unsigned argument,
                      const unsigned char **bytes, size_t *length) {
    if (argument > RXA_COUNT(frame) || RXA_TYPE(frame, argument) != RXT_BINARY)
        return 0;
    void *series = RXA_SERIES(frame, argument);
    if (!series || RL_SERIES(series, RXI_SER_WIDE) != 1) return 0;
    size_t tail = (size_t)RL_SERIES(series, RXI_SER_TAIL);
    size_t index = RXA_INDEX(frame, argument);
    if (index > tail || tail > BYTE_LIMIT) return 0;
    const unsigned char *data = (const unsigned char *)(uintptr_t)RL_SERIES(series, RXI_SER_DATA);
    if (!data && tail != 0) return 0;
    *length = tail - index;
    *bytes = data ? data + index : (const unsigned char *)"";
    return 1;
}

static char *decimal_copy(RXIFRM *frame, unsigned argument) {
    const unsigned char *bytes;
    size_t length;
    if (!bytes_view(frame, argument, &bytes, &length) || length == 0) return NULL;
    size_t first = bytes[0] == '-' ? 1U : 0U;
    if (first == length) return NULL;
    for (size_t i = first; i < length; ++i)
        if (bytes[i] < '0' || bytes[i] > '9') return NULL;
    char *copy = malloc(length + 1U);
    if (!copy) return NULL;
    memcpy(copy, bytes, length);
    copy[length] = '\0';
    return copy;
}

static int finish_binary(RXIFRM *frame, void *series, const unsigned char *bytes, size_t length) {
    if (length > BYTE_LIMIT || RL_SERIES(series, RXI_SER_TAIL) != 0 ||
        RL_SERIES(series, RXI_SER_WIDE) != 1 || RL_SERIES(series, RXI_SER_SIZE) < length + 1U)
        return fail(frame, "Reserved byte result capacity is invalid");
    /* The caller reserved complete capacity before acquiring native resources.
     * Tagged Expand_Series only reallocates if (tail+1+delta)*wide > rest*wide.
     * For each delta1 append below, prove tail==i, wide1 and i+2<=rest before
     * Set_Char invokes that function. Its reserved branch moves the terminator
     * and increments tail; it cannot take the allocation branch. */
    for (size_t i = 0; i < length; ++i) {
        if (RL_SERIES(series, RXI_SER_TAIL) != i || RL_SERIES(series, RXI_SER_WIDE) != 1 ||
            RL_SERIES(series, RXI_SER_SIZE) < i + 2U)
            return fail(frame, "Byte append cannot prove its reserved fast path");
        RL_SET_CHAR(series, (u32)i, bytes[i]);
    }
    RXA_TYPE(frame, 1) = RXT_BINARY;
    RXA_SERIES(frame, 1) = series;
    RXA_INDEX(frame, 1) = 0;
    return RXR_VALUE;
}

static int arithmetic(int command, RXIFRM *frame) {
    if (RXA_COUNT(frame) != 2) return fail(frame, "Two decimal binary operands required");
    const unsigned char *a_bytes, *b_bytes;
    size_t a_length, b_length;
    if (!bytes_view(frame, 1, &a_bytes, &a_length) || !bytes_view(frame, 2, &b_bytes, &b_length))
        return fail(frame, "Two signed-range byte operands required");
    size_t capacity;
    switch (command) {
        case CMD_ADD: case CMD_SUBTRACT:
            capacity = (a_length > b_length ? a_length : b_length) + 2U;
            break;
        case CMD_MULTIPLY: capacity = a_length + b_length + 1U; break;
        case CMD_FLOOR_DIVIDE: capacity = a_length + 2U; break;
        case CMD_COMPARE: capacity = 0; break;
        default: return fail(frame, "Unknown arithmetic operation");
    }
    if (capacity > BYTE_LIMIT) return fail(frame, "Decimal result reserve exceeds signed series range");
    /* VM OOM may escape nonlocally here. No native heap/GMP resource is owned
     * yet. The copies below reacquire input views after possible storage moves. */
    void *series = command == CMD_COMPARE ? NULL : RL_MAKE_BINARY((u32)capacity);
    char *a_text = decimal_copy(frame, 1);
    char *b_text = decimal_copy(frame, 2);
    if (!a_text || !b_text) {
        free(a_text); free(b_text);
        return fail(frame, "Invalid signed decimal binary or native allocation failure");
    }
    mpz_t a, b, result;
    mpz_inits(a, b, result, NULL);
    int valid = mpz_set_str(a, a_text, 10) == 0 && mpz_set_str(b, b_text, 10) == 0;
    free(a_text); free(b_text);
    if (!valid || (command == CMD_FLOOR_DIVIDE && mpz_sgn(b) == 0)) {
        mpz_clears(a, b, result, NULL);
        return fail(frame, "Invalid decimal value or zero divisor");
    }
    if (command == CMD_COMPARE) {
        int compared = mpz_cmp(a, b);
        mpz_clears(a, b, result, NULL);
        RXA_TYPE(frame, 1) = RXT_INTEGER;
        RXA_INT64(frame, 1) = (compared > 0) - (compared < 0);
        return RXR_VALUE;
    }
    switch (command) {
        case CMD_ADD: mpz_add(result, a, b); break;
        case CMD_SUBTRACT: mpz_sub(result, a, b); break;
        case CMD_MULTIPLY: mpz_mul(result, a, b); break;
        case CMD_FLOOR_DIVIDE: mpz_fdiv_q(result, a, b); break;
        default:
            mpz_clears(a, b, result, NULL);
            return fail(frame, "Unknown arithmetic operation");
    }
    size_t digits = mpz_sizeinbase(result, 10);
    if (digits > BYTE_LIMIT - 3U) {
        mpz_clears(a, b, result, NULL);
        return fail(frame, "Decimal result exceeds Rebol series range");
    }
    char *text = malloc(digits + 3U);
    if (!text) {
        mpz_clears(a, b, result, NULL);
        return fail(frame, "Unable to allocate decimal result");
    }
    mpz_get_str(text, 10, result);
    mpz_clears(a, b, result, NULL);
    size_t length = strlen(text);
    if (length > capacity) {
        free(text);
        return fail(frame, "Decimal result exceeds its proven reserve");
    }
    int returned = finish_binary(frame, series, (const unsigned char *)text, length);
    free(text);
    return returned;
}

static int find_bytes(RXIFRM *frame) {
    const unsigned char *pattern, *subject;
    size_t pattern_length, subject_length;
    if (RXA_COUNT(frame) != 3 || RXA_TYPE(frame, 3) != RXT_INTEGER ||
        !bytes_view(frame, 1, &pattern, &pattern_length) ||
        !bytes_view(frame, 2, &subject, &subject_length))
        return fail(frame, "Pattern, subject binaries and integer offset required");
    int64_t start = RXA_INT64(frame, 3);
    if (start < 1 || (uint64_t)start > (uint64_t)subject_length + 1U)
        return fail(frame, "Match offset outside byte subject");
    int error;
    PCRE2_SIZE error_offset;
    /* Ordinary case-sensitive byte mode, explicit LF newline semantics,
     * default library allocator, no JIT and no VM callbacks/allocations. */
    pcre2_compile_context *compile = pcre2_compile_context_create(NULL);
    if (!compile) return fail(frame, "Unable to allocate PCRE2 compile context");
    if (pcre2_set_newline(compile, PCRE2_NEWLINE_LF) != 0) {
        pcre2_compile_context_free(compile);
        return fail(frame, "Unable to select LF byte newline semantics");
    }
    pcre2_code *code = pcre2_compile(pattern, pattern_length, 0, &error, &error_offset, compile);
    pcre2_compile_context_free(compile);
    if (!code) return fail(frame, "PCRE2 pattern compilation failed");
    pcre2_match_data *match = pcre2_match_data_create_from_pattern(code, NULL);
    if (!match) {
        pcre2_code_free(code);
        return fail(frame, "Unable to allocate PCRE2 match data");
    }
    int found = pcre2_match(code, subject, subject_length, (PCRE2_SIZE)(start - 1), 0, match, NULL);
    if (found == PCRE2_ERROR_NOMATCH) {
        pcre2_match_data_free(match); pcre2_code_free(code);
        return RXR_NONE;
    }
    if (found <= 0) {
        pcre2_match_data_free(match); pcre2_code_free(code);
        return fail(frame, "PCRE2 matching failed");
    }
    PCRE2_SIZE *positions = pcre2_get_ovector_pointer(match);
    PCRE2_SIZE begin = positions[0], end = positions[1];
    int valid = begin >= (PCRE2_SIZE)(start - 1) && begin <= end && end <= subject_length;
    pcre2_match_data_free(match); pcre2_code_free(code);
    if (!valid) return fail(frame, "Invalid PCRE2 byte offsets");
    /* No VM allocation occurred while the subject was borrowed. All library
     * temporaries are freed before dispatch constructs the returned block. */
    RXA_COUNT(frame) = 2;
    RXA_TYPE(frame, 1) = RXA_TYPE(frame, 2) = RXT_INTEGER;
    RXA_INT64(frame, 1) = (int64_t)begin + 1;
    RXA_INT64(frame, 2) = (int64_t)end + 1;
    return RXR_BLOCK;
}

static int read_stdin(RXIFRM *frame) {
    if (RXA_COUNT(frame) != 0) return fail(frame, "Read-stdin-chunk takes no arguments");
    /* Allocate before consuming any input. Only a stack buffer is live across
     * the VM boundary; no native heap buffer can leak on its OOM trap. */
    void *series = RL_MAKE_BINARY(IO_CHUNK);
    unsigned char bytes[IO_CHUNK];
    ssize_t count;
    for (;;) {
        count = read(STDIN_FILENO, bytes, sizeof bytes);
        if (count < 0 && errno == EINTR) continue;
        break;
    }
    if (count < 0) return fail(frame, "Stdin byte read failed");
    if (count == 0) return RXR_NONE;
    return finish_binary(frame, series, bytes, (size_t)count);
}

static int write_stdout(RXIFRM *frame) {
    const unsigned char *bytes;
    size_t length;
    if (RXA_COUNT(frame) != 1 || !bytes_view(frame, 1, &bytes, &length))
        return fail(frame, "Write-stdout requires an exact byte binary");
    size_t done = 0;
    while (done < length) {
        ssize_t count = write(STDOUT_FILENO, bytes + done, length - done);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) return fail(frame, "Stdout byte write failed");
        done += (size_t)count;
    }
    return RXR_UNSET;
}

static int format_fixed(RXIFRM *frame) {
    if (RXA_COUNT(frame) != 2 || RXA_TYPE(frame, 1) != RXT_DECIMAL || RXA_TYPE(frame, 2) != RXT_INTEGER)
        return fail(frame, "Decimal and integer precision required");
    double value = RXA_DEC64(frame, 1);
    int64_t digits = RXA_INT64(frame, 2);
    if (!isfinite(value) || digits < 0 || digits > 18)
        return fail(frame, "Finite decimal and precision in0..18 required");
    /* General formatting only: no scaling, benchmark reduction or recurrence.
     * The future child must bind LC_ALL=C and ordinary FPU rounding receipts. */
    char text[512];
    int length = snprintf(text, sizeof text, "%.*f", (int)digits, value);
    if (length < 0 || (size_t)length >= sizeof text) return fail(frame, "Decimal formatting exceeded stack capacity");
    /* Only stack text is live when this VM allocation can nonlocally trap. */
    void *series = RL_MAKE_BINARY((u32)length);
    return finish_binary(frame, series, (const unsigned char *)text, (size_t)length);
}

RXIEXT const char *RX_Init(int options, RL_LIB *library) {
    (void)options;
    if (!library || !CHECK_STRUCT_ALIGN || sizeof(RXIARG) != 16 || sizeof(double) != 8) return NULL;
    RL = library;
    REBYTE version[6] = {0};
    RL_VERSION(version);
    if (version[1] != 3 || version[2] != 22 || version[3] != 1) { RL = NULL; return NULL; }
    return module_source;
}

RXIEXT int RX_Quit(int options) { (void)options; RL = NULL; return 0; }

RXIEXT int RX_Call(int command, RXIFRM *frame, void *context) {
    (void)context;
    if (!RL || !frame) return RXR_NO_COMMAND;
    if (command >= CMD_ADD && command <= CMD_COMPARE) return arithmetic(command, frame);
    switch (command) {
        case CMD_FIND: return find_bytes(frame);
        case CMD_READ: return read_stdin(frame);
        case CMD_WRITE: return write_stdout(frame);
        case CMD_FORMAT: return format_fixed(frame);
        default: return RXR_NO_COMMAND;
    }
}
