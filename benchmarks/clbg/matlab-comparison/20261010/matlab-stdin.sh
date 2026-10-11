#!/bin/sh
# Preserve redirected input before MATLAB batch mode disconnects descriptor 0.
exec 3<&0
exec "$@"
