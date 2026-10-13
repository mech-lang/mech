/* Newly owned generic attach target and read-only kernel identity query.
 * No Julia, canary, benchmark algorithm, permission change or helper child. */
#include <errno.h>
#include <inttypes.h>
#include <libproc.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/proc_info.h>
#include <unistd.h>

static void json_string(const char *text) {
    putchar('"');
    for (const unsigned char *p = (const unsigned char *)text; *p; ++p) {
        if (*p == '"' || *p == '\\') printf("\\%c", *p);
        else if (*p < 32) printf("\\u%04x", *p);
        else putchar(*p);
    }
    putchar('"');
}

static int identity(pid_t pid, const char *role, const char *nonce) {
    struct proc_bsdinfo info = {0};
    char path[PROC_PIDPATHINFO_MAXSIZE] = {0};
    errno = 0;
    int info_bytes = proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, sizeof info);
    int info_errno = errno;
    errno = 0;
    int path_bytes = proc_pidpath(pid, path, sizeof path);
    int path_errno = errno;
    errno = 0;
    pid_t pgid = getpgid(pid);
    int pgid_errno = errno;
    if (info_bytes != (int)sizeof info || path_bytes <= 0 || pgid < 0 ||
        info.pbi_pid != (uint32_t)pid || info.pbi_pgid != (uint32_t)pgid ||
        !memchr(path, 0, sizeof path)) {
        fprintf(stderr, "kernel identity incomplete: pid=%d info=%d/%d path=%d/%d pgid=%d/%d\n",
                (int)pid, info_bytes, info_errno, path_bytes, path_errno, (int)pgid, pgid_errno);
        return 2;
    }
    printf("{\"role\":"); json_string(role);
    printf(",\"nonce\":"); json_string(nonce);
    printf(",\"observer_pid\":%d,\"pid\":%u,\"ppid\":%u,\"pgid\":%u,"
           "\"birth_seconds\":%" PRIu64 ",\"birth_microseconds\":%" PRIu64 ","
           "\"bsd_status\":%u,\"bsd_info_bytes\":%d,\"bsd_info_size\":%zu,"
           "\"proc_pidpath_return\":%d,\"executable_path\":",
           (int)getpid(), info.pbi_pid, info.pbi_ppid, info.pbi_pgid,
           info.pbi_start_tvsec, info.pbi_start_tvusec, info.pbi_status,
           info_bytes, sizeof info, path_bytes);
    json_string(path); puts("}");
    return fflush(stdout) == 0 ? 0 : 3;
}

__attribute__((noinline)) int calibration_probe(int input) {
    volatile int value = input;
    return value + 1;
}

int main(int argc, char **argv) {
    if (argc == 3 && strcmp(argv[1], "--identity") == 0) {
        char *end = NULL;
        errno = 0;
        long requested = strtol(argv[2], &end, 10);
        if (errno || !end || *end || requested <= 0 || requested > INT_MAX) return 4;
        return identity((pid_t)requested, "pre-exec-kernel-query", "");
    }
    if (argc == 3 && strcmp(argv[1], "--run") == 0) {
        for (const char *p = argv[2]; *p; ++p)
            if (!(*p >= 'a' && *p <= 'z') && !(*p >= '0' && *p <= '9') && *p != '-') return 5;
        if (!*argv[2] || strlen(argv[2]) > 96) return 5;
        int result = identity(getpid(), "post-exec-self-identity", argv[2]);
        if (result) return result;
        return calibration_probe(41) == 42 ? 0 : 6;
    }
    return 7;
}
