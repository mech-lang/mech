#include "deploy.h"
#include <stdio.h>
#include <inttypes.h>
#include <mach/mach_time.h>
#include <string.h>
#include <sys/types.h>

static int fail(const char *message) {
    fputs(message, stderr);
    fputc('\n', stderr);
    return 1;
}

static uint8_t *read_input(const char *path, size_t *size) {
    FILE *file = fopen(path, "rb");
    if (!file)
        return NULL;
    off_t length = -1;
    if (fseeko(file, 0, SEEK_END) == 0)
        length = ftello(file);
    if (length < 0 || (uintmax_t)length > SIZE_MAX || fseeko(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return NULL;
    }
    *size = (size_t)length;
    uint8_t *bytes = mi_malloc(*size ? *size : 1);
    int read_ok = bytes && fread(bytes, 1, *size, file) == *size;
    int close_ok = fclose(file) == 0;
    if (read_ok && close_ok)
        return bytes;
    mi_free(bytes);
    return NULL;
}

/*
 * Time one in-process batch per command with the same output ownership
 * contract as the deployment CLI. Input loading and protocol I/O are outside.
 */
static int measure(const uint8_t *input, size_t len) {
    mach_timebase_info_data_t units;
    if (mach_timebase_info(&units) != KERN_SUCCESS)
        return fail("could not read mach timebase");
    if (printf("READY %" PRIu32 " %" PRIu32 "\n", units.numer, units.denom) < 0
        || fflush(stdout) != 0)
        return fail("could not write ready");

    uint64_t count;
    while (scanf("%" SCNu64, &count) == 1 && count != 0) {
        uint64_t output_bytes = 0;
        uint64_t before = mach_absolute_time();
        for (uint64_t i = 0; i < count; ++i) {
            struct deploy_buffer output = {0};
            if (deploy_encode(input, len, &output) != 0)
                return fail("compression failed");
            output_bytes += output.len;
            deploy_release(&output);
        }
        uint64_t elapsed = mach_absolute_time() - before;
        if (printf("%" PRIu64 " %" PRIu64 "\n", elapsed, output_bytes) < 0
            || fflush(stdout) != 0)
            return fail("could not write timing");
    }
    return 0;
}

static int emit(const uint8_t *input, size_t len) {
    struct deploy_buffer output = {0};
    if (deploy_encode(input, len, &output) != 0)
        return fail("compression failed");
    int ok = output.data && output.len && output.len <= output.cap
        && fwrite(output.data, 1, output.len, stdout) == output.len;
    deploy_release(&output);
    return ok && fflush(stdout) == 0 ? 0 : fail("could not write compressed output");
}

int main(int argc, char **argv) {
    int check = argc == 3 && strcmp(argv[2], "check") == 0;
    if (argc != 2 && !check)
        return fail("expected one input file and an optional check");
    size_t len;
    uint8_t *input = read_input(argv[1], &len);
    if (!input)
        return fail("could not read input");
    int status = check ? emit(input, len) : measure(input, len);
    mi_free(input);
    return status;
}
