GilRs Core
==========

[![pipeline status](https://gitlab.com/gilrs-project/gilrs/badges/master/pipeline.svg)](https://gitlab.com/gilrs-project/gilrs-core/commits/master)
[![Minimum rustc version](https://img.shields.io/badge/rustc-1.31.1+-yellow.svg)](https://gitlab.com/gilrs-project/gilrs)

This library is minimal event based abstraction for working with gamepads. If
you are looking for something more hight level, take a look at `gilrs` crate.

Platform specific notes
======================

Linux
-----

On Linux, GilRs read (and write, in case of force feedback) directly from
appropriate `/dev/input/event*` file. This mean that user have to have read and
write access to this file.  On most distros it shouldn't be a problem, but if
it is, you will have to create udev rule.

To build GilRs, you will need pkg-config and libudev .pc file. On some
distributions this file is packaged in separate archive (for example
`libudev-dev` in Debian).

License
=======

This project is licensed under the terms of both the Apache License (Version
2.0) and the MIT license. See LICENSE-APACHE and LICENSE-MIT for details.
