#include "deploy.h"
#include "libdeflate.h"

int deploy_encode(const uint8_t *input, size_t input_len, struct deploy_buffer *output) {
    libdeflate_set_memory_allocator(mi_malloc, mi_free);
    struct libdeflate_compressor *encoder = libdeflate_alloc_compressor(12);
    if (!encoder)
        return 1;
    size_t capacity = libdeflate_deflate_compress_bound(encoder, input_len);
    uint8_t *bytes = capacity ? mi_malloc(capacity) : NULL;
    size_t size = bytes ? libdeflate_deflate_compress(encoder, input, input_len, bytes, capacity) : 0;
    libdeflate_free_compressor(encoder);
    if (!size) {
        mi_free(bytes);
        return 1;
    }
    *output = (struct deploy_buffer){bytes, size, capacity};
    return 0;
}

void deploy_release(struct deploy_buffer *output) {
    mi_free(output->data);
    *output = (struct deploy_buffer){0};
}
