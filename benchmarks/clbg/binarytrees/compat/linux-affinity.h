// macOS has no Linux sched_getaffinity API. This study imposes no affinity mask;
// expose its online logical CPUs for the upstream sources' worker enumeration.
// The tree and allocator algorithms remain the unchanged pinned source.
#include <unistd.h>
#include <stddef.h>
typedef struct { unsigned long bits; } cpu_set_t;
#define CPU_ZERO(set) ((set)->bits = 0)
#define CPU_ISSET(cpu, set) ((cpu) < 64 && (((set)->bits >> (cpu)) & 1))
static inline int sched_getaffinity(int pid, size_t length, cpu_set_t *set) {
    (void)pid;
    if (length < sizeof(*set)) return -1;
    long count = sysconf(_SC_NPROCESSORS_ONLN);
    set->bits = count >= 64 ? ~0UL : count > 0 ? (1UL << count) - 1 : 0;
    return 0;
}
