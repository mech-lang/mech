/* Source-only proposal. No compiler, linker or runtime has executed this file.
 * Link to a reviewed PackageCompiler create_library bundle exporting the
 * @ccallable clbg_entry and its pinned init_julia/shutdown_julia functions.
 * This glue performs no workload, Julia source evaluation or package loading.
 */
#include <stdint.h>
#include <stdio.h>
#include <string.h>

extern void init_julia(int argc, char **argv);
extern void shutdown_julia(int retcode);
extern int32_t clbg_entry(int32_t argc, char **argv);

int main(int argc, char **argv)
{
    static const char *const required[] = {
        "--compile=no", "--threads=8,0", "--startup-file=no",
        "--history-file=no", "-O3"
    };
    int boundary = -1;
    for (int i = 1; i < argc; ++i) {
        if (strcmp(argv[i], "--julia-args") == 0) {
            if (boundary != -1) {
                fputs("Duplicate Julia option boundary\n", stderr);
                return 64;
            }
            boundary = i;
        }
    }
    if (boundary < 1 || argc - boundary - 1 != 5) {
        fputs("Exact reviewed Julia runtime options required\n", stderr);
        return 64;
    }
    for (int i = 0; i < 5; ++i) {
        if (strcmp(argv[boundary + i + 1], required[i]) != 0) {
            fputs("Unexpected Julia runtime option\n", stderr);
            return 64;
        }
    }
    /* The temporary option argv includes argv[0], followed by the fixed policy.
     * Input arguments stay in the original array and are passed at runtime.
     */
    char *julia_argv[7];
    julia_argv[0] = argv[0];
    for (int i = 0; i < 5; ++i)
        julia_argv[i + 1] = argv[boundary + i + 1];
    julia_argv[6] = NULL;
    init_julia(6, julia_argv);
    int32_t result = clbg_entry((int32_t)(boundary - 1), argv + 1);
    shutdown_julia((int)result);
    return (int)result;
}
