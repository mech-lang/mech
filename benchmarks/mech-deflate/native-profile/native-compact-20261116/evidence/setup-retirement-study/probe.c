#include <assert.h>
#include <mach/mach.h>
#include <stdint.h>
#include <stdlib.h>
#include <unistd.h>

extern void *mi_malloc(size_t);
extern void mi_free(void *);

struct probe_vm {
    uint64_t faults, pageins, cow_faults, unix_calls, mach_calls;
};

void probe_vm_read(struct probe_vm *out) {
    task_events_info_data_t value;
    mach_msg_type_number_t count = TASK_EVENTS_INFO_COUNT;
    assert(task_info(mach_task_self(), TASK_EVENTS_INFO, (task_info_t)&value, &count) == KERN_SUCCESS);
    *out = (struct probe_vm){(uint32_t)value.faults, (uint32_t)value.pageins,
        (uint32_t)value.cow_faults, (uint32_t)value.syscalls_unix, (uint32_t)value.syscalls_mach};
}

uint64_t probe_page_bytes(void) { return (uint64_t)getpagesize(); }

struct probe_alloc {
    uint64_t requests, requested_bytes, releases;
};

static struct probe_alloc allocations;

void probe_alloc_reset(void) { allocations = (struct probe_alloc){0}; }
void probe_alloc_read(struct probe_alloc *out) { *out = allocations; }

void *probe_c_malloc(size_t bytes) {
    ++allocations.requests;
    allocations.requested_bytes += bytes;
    return mi_malloc(bytes);
}

void probe_c_free(void *pointer) {
    if (pointer) ++allocations.releases;
    mi_free(pointer);
}
