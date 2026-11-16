// Diagnostic-only own-process PC sampling. No compiler or algorithm changes.
#include <mach/mach.h>
#include <mach/arm/thread_status.h>
#include <mach-o/dyld.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <dlfcn.h>
#include <string.h>

#define CAPACITY 60000
typedef struct { uint64_t at_ns, thread, pc, lr, sp, fp, phase; } Record;
static Record *records;
static size_t used;
static pthread_t sampler;
static thread_t initial_target;
static _Atomic int enabled, finished, ready;
static uint64_t scans, attempts, suspended, resumed, states, stopped;
static _Atomic uint64_t phase_epoch;
static uint64_t phase_changed;
static uint64_t suspend_failures, state_failures, resume_terminated, full;
static uint64_t first_ns, last_ns;
static int warm_ok;

static uint64_t now_ns(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (uint64_t)t.tv_sec * 1000000000ULL + (uint64_t)t.tv_nsec;
}

// Register snapshots only. No user thread suspension and no stack memory walk.
// Kernel-rejected or disappearing endpoints are simply absent observations.
static int capture(thread_t target, Record *record, int require_enabled) {
    if (require_enabled && !atomic_load_explicit(&enabled, memory_order_acquire)) { stopped++; return 0; }
    arm_thread_state64_t state;
    mach_msg_type_number_t count = ARM_THREAD_STATE64_COUNT;
    attempts++;
    kern_return_t status = thread_get_state(target, ARM_THREAD_STATE64, (thread_state_t)&state, &count);
    if (status != KERN_SUCCESS || count != ARM_THREAD_STATE64_COUNT) { state_failures++; return 0; }
    states++;
    record->thread = target;
    record->pc = arm_thread_state64_get_pc(state);
    record->lr = arm_thread_state64_get_lr(state);
    record->sp = arm_thread_state64_get_sp(state);
    record->fp = arm_thread_state64_get_fp(state);
    return 1;
}

static void *sample_main(void *unused) {
    (void)unused;
    thread_t own = mach_thread_self();
    Record probe = {0};
    warm_ok = capture(initial_target, &probe, 0);
    atomic_store_explicit(&ready, 1, memory_order_release);
    const struct timespec interval = {.tv_sec=0, .tv_nsec=1000000};
    while (!atomic_load_explicit(&finished, memory_order_acquire)) {
        nanosleep(&interval, NULL);
        if (!warm_ok || !atomic_load_explicit(&enabled, memory_order_acquire)) continue;
        uint64_t epoch = atomic_load_explicit(&phase_epoch, memory_order_acquire);
        if (!(epoch & 1)) continue;
        uint64_t timestamp = now_ns();
        if (!first_ns) first_ns = timestamp;
        if (timestamp - first_ns >= 15000000000ULL) continue;
        thread_act_array_t threads = NULL;
        mach_msg_type_number_t count = 0;
        if (task_threads(mach_task_self(), &threads, &count) != KERN_SUCCESS) continue;
        scans++;
        for (mach_msg_type_number_t i = 0; i < count; i++) {
            if (threads[i] != own) {
                if (used == CAPACITY) full++;
                else {
                    Record record = {0};
                    if (capture(threads[i], &record, 1)) {
                        if (epoch == atomic_load_explicit(&phase_epoch, memory_order_acquire)) {
                            record.at_ns = timestamp;
                            record.phase = epoch;
                            records[used++] = record;
                            last_ns = timestamp;
                        } else phase_changed++;
                    }
                }
            }
            mach_port_deallocate(mach_task_self(), threads[i]);
        }
        vm_deallocate(mach_task_self(), (vm_address_t)threads, count * sizeof(thread_t));
    }
    mach_port_deallocate(mach_task_self(), own);
    return NULL;
}

int mpc_init(void) {
    _Static_assert(ATOMIC_INT_LOCK_FREE == 2 && ATOMIC_LLONG_LOCK_FREE == 2, "sampling flags and epochs must be lock free");
    initial_target = pthread_mach_thread_np(pthread_self());
    records = calloc(CAPACITY, sizeof(Record));
    if (!initial_target || !records) return -1;
    for (size_t i = 0; i < CAPACITY; i++) records[i].pc = 0;
    int status = pthread_create(&sampler, NULL, sample_main, NULL);
    if (status) return status;
    while (!atomic_load_explicit(&ready, memory_order_acquire)) { }
    return warm_ok ? 0 : -2;
}
uint64_t mpc_phase(int active) {
    uint64_t before = atomic_fetch_add_explicit(&phase_epoch, 1, memory_order_acq_rel);
    if (((before + 1) & 1) != !!active) abort();
    return before + 1;
}
void mpc_enable(int value) { atomic_store_explicit(&enabled, value, memory_order_release); }
int mpc_finish(const char *binary_path, const char *stats_path) {
    mpc_enable(0);
    atomic_store_explicit(&finished, 1, memory_order_release);
    pthread_join(sampler, NULL);
    FILE *binary = fopen(binary_path, "wb");
    if (!binary) return -1;
    size_t written = fwrite(records, sizeof(Record), used, binary);
    fclose(binary);
    FILE *stats = fopen(stats_path, "w");
    if (!stats) return -2;
    fprintf(stats, "{\"records\":%zu,\"record_bytes\":%zu,\"image_base\":%llu,\"capacity\":%d,\"scans\":%llu,\"attempts\":%llu,\"suspended\":%llu,\"resumed\":%llu,\"states\":%llu,\"stopped\":%llu,\"suspend_failures\":%llu,\"state_failures\":%llu,\"resume_terminated\":%llu,\"full\":%llu,\"first_ns\":%llu,\"last_ns\":%llu,\"last_phase_epoch\":%llu,\"phase_changed\":%llu,\"post_json_only\":true}\n",
        used, sizeof(Record), (unsigned long long)(uintptr_t)_dyld_get_image_header(0), CAPACITY,
        (unsigned long long)scans, (unsigned long long)attempts, (unsigned long long)suspended,
        (unsigned long long)resumed, (unsigned long long)states, (unsigned long long)stopped,
        (unsigned long long)suspend_failures, (unsigned long long)state_failures,
        (unsigned long long)resume_terminated, (unsigned long long)full,
        (unsigned long long)first_ns, (unsigned long long)last_ns,
        (unsigned long long)atomic_load_explicit(&phase_epoch, memory_order_acquire),
        (unsigned long long)phase_changed);
    fclose(stats);
    // Resolve every captured PC while this exact process and all its loaded
    // images are still alive. This includes system wait/allocator PCs absent
    // from the executable-only symbol table. Resolution occurs after sampling.
    size_t path_size = strlen(binary_path) + 32;
    char *symbol_path = malloc(path_size);
    if (!symbol_path) return -4;
    snprintf(symbol_path, path_size, "%s.symbols.tsv", binary_path);
    FILE *symbols = fopen(symbol_path, "w");
    free(symbol_path);
    if (!symbols) return -5;
    for (size_t i = 0; i < used; i++) {
        Dl_info info = {0};
        int found = dladdr((const void *)(uintptr_t)records[i].pc, &info);
        fprintf(symbols, "%llx\t%llx\t%llx\t%s\t%s\n",
            (unsigned long long)records[i].pc,
            (unsigned long long)(uintptr_t)info.dli_fbase,
            (unsigned long long)(uintptr_t)info.dli_saddr,
            found && info.dli_fname ? info.dli_fname : "-",
            found && info.dli_sname ? info.dli_sname : "-");
    }
    fclose(symbols);
    free(records);
    return written == used ? 0 : -3;
}
