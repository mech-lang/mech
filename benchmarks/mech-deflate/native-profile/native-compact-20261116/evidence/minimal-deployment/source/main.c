#include "deploy.h"
#include <stdio.h>
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

int main(int argc, char **argv) {
    if (argc != 2)
        return fail("expected one input file");
    size_t input_len = 0;
    uint8_t *input = read_input(argv[1], &input_len);
    if (!input)
        return fail("could not read input");
    struct deploy_buffer output = {0};
    int status = deploy_encode(input, input_len, &output);
    mi_free(input);
    if (status != 0)
        return fail("compression failed");
    int write_ok = output.data && output.len && output.len <= output.cap
        && fwrite(output.data, 1, output.len, stdout) == output.len;
    deploy_release(&output);
    if (!write_ok || fflush(stdout) != 0)
        return fail("could not write compressed output");
    return 0;
}
