# Vase Multithreaded Compiler
This is an in-development attempt at making a multi-threaded C compiler in Rust.

## Rust Version
This project requires Rust nightly. To use `cargo fmt` you must also
install `rustfmt` 2.0.0.rc-2.

## Multithreading
The multi-threaded nature of the compiler has informed much of its
design. Many structures use locks or lock-free utilities to permit
threads to work in parallel.

The current plan is for each stage of compilation to be parallelized:
* Lexer: For each source and header file.
* Parser: For each source file.
* Analyzer: For each source file.
* Generator: For each source file.

Due to the all-at-once nature of the compiler, more memory will be used
than other compilers.

## License

Right now the code is licensed under GPLv3 or later (see LICENSE).
However, I do wish to retain the ability to re-license the whole
project should I desire to.
