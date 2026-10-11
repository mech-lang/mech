package CLBGLinuxCPUInfo;
use strict;
use warnings;

# Preserve the normal open operation, redirecting only Linux's CPU probe.
BEGIN {
    *CORE::GLOBAL::open = sub (*;$@) {
        if (@_ == 2 && $_[1] eq '</proc/cpuinfo') {
            return CORE::open($_[0], '<', $ENV{CLBG_CPUINFO_FILE});
        }
        return @_ == 2 ? CORE::open($_[0], $_[1])
                      : CORE::open($_[0], $_[1], $_[2]);
    };
}
1;
