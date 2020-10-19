# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
* The Rune project now has a Code of Conduct ([#12]).
* Support for bitwise operations on numbers ([#13], [#20]).
* Book now has support for highlighting `rune` blocks ([#14]).
* Preliminary support for modules without visibility ([#16], [#17]).
* Debug information for function variable names now reflect source ([#24]).
* Initial support for macros ([#29], [#30], [#31], [#114], [#135], [#136],
  [#137], [#138], [#141], [#142], [#143], [#144]).
* Add cargo build cache ([#36]) (thanks [shekohex]!).
* Rust `quote!` macro for Rune macro authors ([#34]).
* Support for object- and tuple-like field assignments ([#38], [#39], [#40],
  [#66]).
* Support for lazy evaluation for and/or (`&&` / `||`) ([#50]) (thanks
  [seanchen1991]!).
* Add `AsTokens`, `FromValue`, `ToValue`, and `Spanned` derives ([#41], [#85],
  [#87], [#88], [#113]).
* Visual studio code extension with syntax highlighting and basic language
  server ([#46], [#47], [#48], [#60], [#74]) (thanks [killercup]!).
  * As-you-type building ([#49]).
  * Jump to definitions ([#61]).
  * Multifile project support ([#64]).
  * Automatic downloading of language server binary ([#69]).
* Non-zero exit status on script errors ([#58], [#59]) (thanks [killercup]!).
* Improve CLI by parsing arguments using [`structopt`] ([#51]) (thanks
  [shekohex]!).
* Executing functions in the virtual machine can use external references
  ([#52]).
* Remove unused instruction in `loop` ([#53]) (thanks [genusistimelord]!).
* Tweak module dependencies to use native Rust modules ([#54]) (thanks
  [killercup]!).
* Internal changes to support a future C FFI ([#55]).
* Improving module API ([#56]).
* Extending `http` module to deserialize JSON directly ([#57]) (thanks
  [killercup]!).
* Automatic build releases on tags ([#68]).
* Fixed locals bug with breaking control in the middle of an index get operation
  ([#71]).
* Community site at https://rune-rs.github.io ([#75]).
* Add WASM-based Playground to community site https://rune-rs.github.io ([#77]).
* Support for limiting execution of `rune-wasm` ([#80]).
* Support for modules, imports, re-exports, visibility, and path resolution
  ([#83], [#92], [#98], [#124], [#125], [#128], [#129], [#130], [#131], [#133],
  [#134], [#148], [#155]) (thanks [dillonhicks]!).
* Add WASM support for a couple of showcased rune modules ([#89]).
* Added runtime type information (RTTI) for values in Runestick ([#90], [#112]).
* Add a `rand` module to `rune-modules` ([#100]) (thanks [aspenluxxxy]!).
* Initial support for constant evaluation ([#93], [#94], [#99], [#104], [#105],
  [#106], [#107], [#117], [#122], [#123], [#153]).
* Add `Args` implementation for `Vec` ([#147]) (thanks [MinusGix]!).
* Export a `Function` variant called `SyncFunction` that is thread-safe ([#149],
  [#151]) (thanks [MinusGix]!).
* Support `move` modifier to async blocks and closures to take ownership of
  values being used ([#152]).
* Basic `Iterator` support ([#156], [#157]) (thanks [MinusGix]!).
* Support for calling protocol functions from native code using `Interface` ([#159]).

### Changed
* Make units more efficient by separating runtime and compile-time metadata ([#24]).
* Change the internal representation of `Item` to be more memory efficient ([#63]).
* Make the implementation of `ParseError` and `CompileError` more consistent ([#65]).
* Remove the `rune-testing` module ([#67]).
* Made evaluation order of index set operations the same as Rust ([#70]).
* Make hashing less error prone ([#72]).
* Various parser changes and tests ([#110]).
* Various internal changes ([#103], [#108], [#109]).
* Parser simplifications ([#120], [#121]).
* Negative literals are handled as expressions ([#132]).
* Syntax for template strings now follows EcmaScript ([#145]).

### Fixed
* Introduced custom highlight.js to fix issue with hidden lines in the book
  ([#10]).
* Semi-colons in blocks weren't required, they now are ([#32]).
* Fixed field assignments ([#38], [#40]) (thanks [MinusGix]!).
* Book typos ([#11], [#18], [#28], [#37]) (thanks [Sparkpin], [seanchen1991],
  [stoically], and [macginitie]!).
* Fix broken book links ([#84], [#86]) (thanks [dillonhicks]!).
* Fix pattern miscompilation ([#62]).
* Fixed bug with Closure optimization where it's being treated as a function
  ([#21], [#22]) (thanks [MinusGix]!).
* Fixed a number of clippy lints ([#35]) (thanks [shekohex]!).
* Fix using closures in literals, like `(0, || 42)` or `#{a: || 42}` ([#78]).
* Shared access guards didn't implement Drop allowing them to leak their guarded
  value ([#119]).

[`structopt`]: https://docs.rs/structopt

[Sparkpin]: https://github.com/Sparkpin
[seanchen1991]: https://github.com/seanchen1991
[stoically]: https://github.com/stoically
[MinusGix]: https://github.com/MinusGix
[shekohex]: https://github.com/shekohex
[macginitie]: https://github.com/macginitie
[genusistimelord]: https://github.com/genusistimelord
[killercup]: https://github.com/killercup
[dillonhicks]: https://github.com/dillonhicks
[aspenluxxxy]: https://github.com/aspenluxxxy

[#10]: https://github.com/rune-rs/rune/issues/10
[#11]: https://github.com/rune-rs/rune/pull/11
[#12]: https://github.com/rune-rs/rune/pull/12
[#13]: https://github.com/rune-rs/rune/pull/13
[#14]: https://github.com/rune-rs/rune/pull/14
[#16]: https://github.com/rune-rs/rune/pull/16
[#17]: https://github.com/rune-rs/rune/pull/17
[#18]: https://github.com/rune-rs/rune/pull/18
[#20]: https://github.com/rune-rs/rune/pull/20
[#21]: https://github.com/rune-rs/rune/issues/21
[#22]: https://github.com/rune-rs/rune/pull/22
[#24]: https://github.com/rune-rs/rune/pull/24
[#28]: https://github.com/rune-rs/rune/pull/28
[#29]: https://github.com/rune-rs/rune/pull/29
[#30]: https://github.com/rune-rs/rune/pull/30
[#31]: https://github.com/rune-rs/rune/pull/31
[#32]: https://github.com/rune-rs/rune/pull/32
[#34]: https://github.com/rune-rs/rune/pull/34
[#35]: https://github.com/rune-rs/rune/pull/35
[#36]: https://github.com/rune-rs/rune/pull/36
[#37]: https://github.com/rune-rs/rune/issues/37
[#38]: https://github.com/rune-rs/rune/pull/38
[#39]: https://github.com/rune-rs/rune/pull/39
[#40]: https://github.com/rune-rs/rune/pull/40
[#41]: https://github.com/rune-rs/rune/pull/41
[#46]: https://github.com/rune-rs/rune/pull/46
[#47]: https://github.com/rune-rs/rune/pull/47
[#48]: https://github.com/rune-rs/rune/pull/48
[#49]: https://github.com/rune-rs/rune/pull/49
[#50]: https://github.com/rune-rs/rune/pull/50
[#51]: https://github.com/rune-rs/rune/pull/51
[#52]: https://github.com/rune-rs/rune/pull/52
[#53]: https://github.com/rune-rs/rune/pull/53
[#54]: https://github.com/rune-rs/rune/pull/54
[#55]: https://github.com/rune-rs/rune/pull/55
[#56]: https://github.com/rune-rs/rune/pull/56
[#57]: https://github.com/rune-rs/rune/pull/57
[#58]: https://github.com/rune-rs/rune/issues/58
[#59]: https://github.com/rune-rs/rune/pull/59
[#60]: https://github.com/rune-rs/rune/pull/60
[#61]: https://github.com/rune-rs/rune/pull/61
[#62]: https://github.com/rune-rs/rune/pull/62
[#63]: https://github.com/rune-rs/rune/pull/63
[#64]: https://github.com/rune-rs/rune/pull/64
[#65]: https://github.com/rune-rs/rune/pull/65
[#66]: https://github.com/rune-rs/rune/pull/66
[#67]: https://github.com/rune-rs/rune/pull/67
[#68]: https://github.com/rune-rs/rune/pull/68
[#69]: https://github.com/rune-rs/rune/pull/69
[#70]: https://github.com/rune-rs/rune/pull/70
[#71]: https://github.com/rune-rs/rune/pull/71
[#72]: https://github.com/rune-rs/rune/pull/72
[#74]: https://github.com/rune-rs/rune/pull/74
[#75]: https://github.com/rune-rs/rune/pull/75
[#77]: https://github.com/rune-rs/rune/pull/77
[#78]: https://github.com/rune-rs/rune/pull/78
[#80]: https://github.com/rune-rs/rune/pull/80
[#83]: https://github.com/rune-rs/rune/pull/83
[#84]: https://github.com/rune-rs/rune/pull/84
[#85]: https://github.com/rune-rs/rune/pull/85
[#86]: https://github.com/rune-rs/rune/pull/86
[#87]: https://github.com/rune-rs/rune/pull/87
[#88]: https://github.com/rune-rs/rune/pull/88
[#89]: https://github.com/rune-rs/rune/pull/89
[#90]: https://github.com/rune-rs/rune/pull/90
[#92]: https://github.com/rune-rs/rune/pull/92
[#93]: https://github.com/rune-rs/rune/pull/93
[#94]: https://github.com/rune-rs/rune/pull/94
[#98]: https://github.com/rune-rs/rune/pull/98
[#99]: https://github.com/rune-rs/rune/pull/99
[#100]: https://github.com/rune-rs/rune/pull/100
[#103]: https://github.com/rune-rs/rune/pull/103
[#104]: https://github.com/rune-rs/rune/pull/104
[#105]: https://github.com/rune-rs/rune/pull/105
[#106]: https://github.com/rune-rs/rune/pull/106
[#107]: https://github.com/rune-rs/rune/pull/107
[#108]: https://github.com/rune-rs/rune/pull/108
[#109]: https://github.com/rune-rs/rune/pull/109
[#110]: https://github.com/rune-rs/rune/pull/110
[#112]: https://github.com/rune-rs/rune/pull/112
[#113]: https://github.com/rune-rs/rune/pull/113
[#114]: https://github.com/rune-rs/rune/pull/114
[#117]: https://github.com/rune-rs/rune/pull/117
[#119]: https://github.com/rune-rs/rune/pull/119
[#120]: https://github.com/rune-rs/rune/pull/120
[#121]: https://github.com/rune-rs/rune/pull/121
[#122]: https://github.com/rune-rs/rune/pull/122
[#123]: https://github.com/rune-rs/rune/pull/123
[#124]: https://github.com/rune-rs/rune/pull/124
[#125]: https://github.com/rune-rs/rune/pull/125
[#128]: https://github.com/rune-rs/rune/pull/128
[#129]: https://github.com/rune-rs/rune/pull/129
[#130]: https://github.com/rune-rs/rune/pull/130
[#131]: https://github.com/rune-rs/rune/pull/131
[#132]: https://github.com/rune-rs/rune/pull/132
[#133]: https://github.com/rune-rs/rune/pull/133
[#134]: https://github.com/rune-rs/rune/pull/134
[#135]: https://github.com/rune-rs/rune/pull/135
[#136]: https://github.com/rune-rs/rune/pull/136
[#137]: https://github.com/rune-rs/rune/pull/137
[#138]: https://github.com/rune-rs/rune/pull/138
[#141]: https://github.com/rune-rs/rune/pull/141
[#142]: https://github.com/rune-rs/rune/pull/142
[#143]: https://github.com/rune-rs/rune/pull/143
[#144]: https://github.com/rune-rs/rune/pull/144
[#145]: https://github.com/rune-rs/rune/pull/145
[#147]: https://github.com/rune-rs/rune/pull/147
[#148]: https://github.com/rune-rs/rune/pull/148
[#149]: https://github.com/rune-rs/rune/pull/149
[#151]: https://github.com/rune-rs/rune/pull/151
[#152]: https://github.com/rune-rs/rune/pull/152
[#153]: https://github.com/rune-rs/rune/pull/153
[#155]: https://github.com/rune-rs/rune/pull/155
[#156]: https://github.com/rune-rs/rune/pull/156
[#157]: https://github.com/rune-rs/rune/pull/157
[#159]: https://github.com/rune-rs/rune/pull/159

[Unreleased]: https://github.com/rune-rs/rune/compare/0.6.16...master
