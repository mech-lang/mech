/* Official trimmed-library route: only the already recorded C ABI is called.
 * This is fresh counted application glue, not the old PackageCompiler driver.
 * No source evaluation, package loading or runtime fallback is added here.
 */
#include <stdint.h>
#include <limits.h>

extern int32_t clbg_entry(int32_t argc, char **argv);

int main(int argc, char **argv)
{
    if (argc < 1 || (uint64_t)(argc - 1) > INT32_MAX)
        return 64;
    return (int)clbg_entry((int32_t)(argc - 1), argv + 1);
}
