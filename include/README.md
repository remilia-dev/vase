# Includes
`/usr/include/` on a Linux system contains *most* of the standard C headers
(and then some). However, it is missing a few headers (I suspect due
to compiler differences). Most of these headers are simple enough to implement
manually. There are two exceptions, that I've taken from Clang:
* `float.h`
* `stdint.h`

# License

Because there is a mix of files here, the license is a bit complicated.

## Dual Licensed

* `iso646.h`
* `stdalign.h`
* `stdarg.h`
* `stdbool.h`
* `stddef.h`
* `stdnoreturn.h`

are dual-licensed under GPLv3-or-later *and* Apache 2.0 with LLVM Exception.

## Single Licensed

* `float.h`
* `stdint.h`

are *only* licensed under Apache 2.0 with LLVM Exception. They are from the
[Clang project](https://clang.llvm.org) and have not been altered.