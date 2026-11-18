#include "deploy.h"
#include "roc_platform_abi.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

RocList roc_compress(RocList input, RocList output);

void *mi_malloc_aligned(size_t size, size_t alignment);
void *mi_realloc(void *pointer, size_t size);
void *mi_realloc_aligned(void *pointer, size_t size, size_t alignment);

static _Noreturn void out_of_memory(void) {
    fputs("out of memory\n", stderr);
    exit(1);
}

void *roc_alloc(size_t size, size_t alignment) {
    size = size ? size : 1;
    void *result = alignment <= _Alignof(max_align_t)
        ? mi_malloc(size) : mi_malloc_aligned(size, alignment);
    if (!result)
        out_of_memory();
    return result;
}

void roc_dealloc(void *pointer, size_t alignment) {
    (void)alignment;
    mi_free(pointer);
}

void *roc_realloc(void *pointer, size_t size, size_t alignment) {
    size = size ? size : 1;
    void *result = alignment <= _Alignof(max_align_t)
        ? mi_realloc(pointer, size) : mi_realloc_aligned(pointer, size, alignment);
    if (!result)
        out_of_memory();
    return result;
}

void roc_dbg(const uint8_t *bytes, size_t length) {
    fwrite(bytes, 1, length, stderr);
}

void roc_crashed(const uint8_t *bytes, size_t length) {
    roc_dbg(bytes, length);
    exit(1);
}

void roc_expect_failed(const uint8_t *bytes, size_t length) {
    roc_crashed(bytes, length);
}

/* Pinned Roc List(U8) ABI. Owned bytes follow a refcount word, normal
   capacities are shifted, and sliced lists tag their original data pointer. */
static RocList owned_list(const void *bytes, size_t length) {
    size_t *count = roc_alloc(sizeof(*count) + length, _Alignof(size_t));
    *count = 1;
    uint8_t *data = (uint8_t *)(count + 1);
    memcpy(data, bytes, length);
    return (RocList){.elements = data, .length = length, .capacity_or_alloc_ptr = length << 1};
}

static void release_list_data(void *allocation_data) {
    if (!allocation_data)
        return;
    size_t *count = (size_t *)allocation_data - 1;
    size_t previous = __atomic_load_n(count, __ATOMIC_RELAXED);
    if (previous && __atomic_fetch_sub(count, 1, __ATOMIC_RELAXED) == 1)
        roc_dealloc(count, _Alignof(size_t));
}

int deploy_encode(const uint8_t *input, size_t input_len, struct deploy_buffer *output) {
    const size_t slice_tag = 1;
    const RocList empty = {0};
    if (input_len > SIZE_MAX / 2)
        return 1;
    /* The native entry consumes its input list. Preserve the caller's borrow. */
    RocList source = input_len ? owned_list(input, input_len) : empty;
    RocList result = roc_compress(source, empty);
    int is_slice = (result.capacity_or_alloc_ptr & slice_tag) != 0;
    void *allocation_data = is_slice
        ? (void *)(result.capacity_or_alloc_ptr & ~slice_tag) : result.elements;
    size_t capacity = is_slice ? result.length : result.capacity_or_alloc_ptr >> 1;
    if (!result.elements || !result.length || result.length > capacity || capacity > SIZE_MAX / 2) {
        release_list_data(allocation_data);
        return 1;
    }
    if (is_slice) {
        result = owned_list(result.elements, result.length);
        release_list_data(allocation_data);
    }
    *output = (struct deploy_buffer){result.elements, result.length, capacity};
    return 0;
}

void deploy_release(struct deploy_buffer *output) {
    release_list_data(output->data);
    *output = (struct deploy_buffer){0};
}
