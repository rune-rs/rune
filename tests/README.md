# Tests for Rune

This project is structured slightly differently than your average Rust testing
project.

Autodiscover is **turned off**, meaning all tests have to be included manually
in [tests/mod.rs]. This is done so that all tests can be compiled into a single
binary which has a significant speedup effect on running them. Especially on
systems which have very slow linkers.

This is also done so that we can provide a few convenient macros in [tests.rs]
which are available implicitly in all test cases.
