#include <dlfcn.h>
#include <mach/arm/thread_status.h>
#include <signal.h>
#include <stdatomic.h>
#include <stdint.h>
#include <sys/time.h>
#include <sys/ucontext.h>

_Static_assert(ATOMIC_POINTER_LOCK_FREE == 2 && ATOMIC_INT_LOCK_FREE == 2,
               "the signal handler only uses lock-free atomics");

enum { SAMPLE_LIMIT = 32768 };
static _Atomic uintptr_t addresses[SAMPLE_LIMIT];
static _Atomic unsigned count;
static _Atomic int inside_call;

static void sample_pc(int signum, siginfo_t *info, void *raw_context) {
    (void)signum;
    (void)info;
    if (!atomic_load_explicit(&inside_call, memory_order_relaxed)) {
        return;
    }
    unsigned i = atomic_load_explicit(&count, memory_order_relaxed);
    if (i == SAMPLE_LIMIT) {
        return;
    }
    const ucontext_t *context = raw_context;
    uintptr_t pc = arm_thread_state64_get_pc(context->uc_mcontext->__ss);
    atomic_store_explicit(&addresses[i], pc, memory_order_relaxed);
    atomic_store_explicit(&count, i + 1, memory_order_relaxed);
}

int mech_probe_pc_start(unsigned microseconds) {
    struct sigaction action = {0};
    action.sa_sigaction = sample_pc;
    action.sa_flags = SA_SIGINFO | SA_RESTART;
    sigemptyset(&action.sa_mask);
    if (sigaction(SIGPROF, &action, 0) != 0) {
        return -1;
    }
    atomic_store_explicit(&count, 0, memory_order_relaxed);
    const struct itimerval interval = {{0, microseconds}, {0, microseconds}};
    return setitimer(ITIMER_PROF, &interval, 0);
}

void mech_probe_pc_scope(int active) {
    atomic_store_explicit(&inside_call, active, memory_order_relaxed);
}

int mech_probe_pc_stop(void) {
    const struct itimerval stopped = {{0, 0}, {0, 0}};
    if (setitimer(ITIMER_PROF, &stopped, 0) != 0) {
        return -1;
    }
    return (int)atomic_load_explicit(&count, memory_order_relaxed);
}

uintptr_t mech_probe_pc_read(unsigned index) {
    return atomic_load_explicit(&addresses[index], memory_order_relaxed);
}

int mech_probe_pc_image(uintptr_t pc, uintptr_t anchor, const char **image, const char **symbol) {
    Dl_info info, executable;
    if (dladdr((void *)pc, &info) == 0 || dladdr((void *)anchor, &executable) == 0 ||
        info.dli_fbase == executable.dli_fbase) {
        return 0;
    }
    *image = info.dli_fname;
    *symbol = info.dli_sname;
    return 1;
}
