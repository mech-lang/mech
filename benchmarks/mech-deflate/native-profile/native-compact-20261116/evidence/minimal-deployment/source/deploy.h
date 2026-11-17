#ifndef MECH_DEFLATE_DEPLOY_H
#define MECH_DEFLATE_DEPLOY_H

#include <stddef.h>
#include <stdint.h>

struct deploy_buffer {
    uint8_t *data;
    size_t len;
    size_t cap;
};

/* Borrow input and return one owned buffer containing a raw DEFLATE stream. */
int deploy_encode(const uint8_t *input, size_t input_len, struct deploy_buffer *output);
void deploy_release(struct deploy_buffer *output);

void *mi_malloc(size_t size);
void mi_free(void *pointer);

#endif
