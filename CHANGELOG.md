# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

[Unreleased]: https://github.com/rune-rs/rune/compare/0.9.0...main

## [0.9.0]

### Changed
* rune-modules now uses tokio 1.x.

### Fixed
* `Vm::async_call` didn't use async completion functions ([#253]) (thanks [Roba1993]!).

[0.9.0]: https://github.com/rune-rs/rune/compare/0.8.0...0.9.0

[Roba1993]: https://github.com/Roba1993

## [0.8.0]

### Added
* Support for `#[test]` annotations ([#218], [#222]) (thanks [tgolsson]!).
* Add `file!()` and `line!()` macros ([#168]) (thanks [tgolsson]!).
* Support for field functions and derives to implement them ([#169], [#170]).
* Support for `crate` in modules ([#172]).
* `std::any` APIs for runtime inspection of types ([#178]) (thanks [tgolsson]!).
* Support for range expressions ([#180]).
* Missing implementations for `FromValue` conversions for `i16` and `u16` ([#235]) (thanks [genusistimelord]!).
* More APIs and iterator-heavy benchmark ([#232]) (thanks [tgolsson]!).
* Added initial benchmarks ([#189]).
* Added cellular automata benchmark ([#220]) (thanks [tgolsson]!).
* Added fibonacci and brainfuck benchmarks ([#193]) (thanks [tgolsson]!).
* Projection APIs for internal `Ref` / `RefMut` ([#211]).
* **Many** API additions and fixes ([#219], [#229], [#233], [#241], [#196], [#199], [#185]) (thanks [tgolsson]!).
* Annotations to improve measuring the performance of individual operations in the VM ([#190]).
* Pattern matching for booleans ([#188]) (thanks [genusistimelord]!).
* Added support for `continue` inside of loops ([#183]).
* Add support for registering and accessing runtime constants ([#239]).
* Support for panicking pattern binding in function arguments ([#195]).
* Added parsing for yet-to-be supported path segments ([#206]).
* Add basic support for threaded execution ([#97]).

### Changed
* Minor changes ([#247], [#208]).
* Improved CLI with `cargo`-like subcommands ([#223]) (thanks [tgolsson]!).
* Compile-time metadata has been simplified ([#163], [#164]).
* Internal compiler improvements ([#173], [#174]).
* Make types used in `Context` iteration APIs public ([#176]).
* Slim down the size of runtime meta ([#177]).
* Change and improve how protocol functions are called ([#210], [#209]).
* Improve performance of runtime hashing ([#191]).
* Improve diagnostics when using an exclusive reference which is not exclusive ([#213]).
* Improve performance by reducing the number of copies generated ([#194]).
* Make compile hooks refcounted for increased flexibility ([#221]).
* Expose LSP server as a modular library for custom uses ([#186]) (thanks [tgolsson]!).
* Improve performance of small objects by using BTreeMap for field storage ([#231]) (thanks [tgolsson]!).
* Report errors and warnings through same diagnostics structure ([#227], [#228]).

### Fixed
* Minor fixes ([#198], [#201]).
* Documentation fixes and improvements ([#248], [#234], [#242]) (thanks [robojumper], [maxmcd], and [hvithrafn]!).
* Fix negative fractional literals ([#184]) (thanks [tgolsson]!).
* Various fixes for use in [OxidizeBot] ([#161]).
* Bug with using wrong protocol for `MUL` and `DIV` ([#167]).
* Add missing macro modules in rune-wasm ([#171]) (thanks [tgolsson]!).
* Fixed buggy visibility checks for paths ([#175]).
* Various fixes and improvements due to [AoC] ([#181], [#187], [#192], [#197], [#203], [#204], [#205], [#216], [#217]).
* Give `SourceLoader` a lifetime ([#245]) (thanks [tgolsson]!).
* Fix miscompilation in struct literals ([#246]) (thanks [robojumper]!).
* Fix miscompilation in pattern matching ([#214]).
* Introduced and fixed binding bug ([#202]).
* Fix so that different variants of the same enum have different equalities ([#215]).
* Make float associated fns associated ([#240]) (thanks [tgolsson]!).
* Bump nanorand to fix incorrect generation of random numbers in `rand` module ([#243]) (thanks [tgolsson]!).
* Fixed broken assembly of more than one `if else` ([#230]).

[#97]: https://github.com/rune-rs/rune/pull/97
[#161]: https://github.com/rune-rs/rune/pull/161
[#163]: https://github.com/rune-rs/rune/pull/163
[#164]: https://github.com/rune-rs/rune/pull/164
[#167]: https://github.com/rune-rs/rune/pull/167
[#168]: https://github.com/rune-rs/rune/pull/168
[#169]: https://github.com/rune-rs/rune/pull/169
[#170]: https://github.com/rune-rs/rune/pull/170
[#171]: https://github.com/rune-rs/rune/pull/171
[#172]: https://github.com/rune-rs/rune/pull/172
[#173]: https://github.com/rune-rs/rune/pull/173
[#174]: https://github.com/rune-rs/rune/pull/174
[#175]: https://github.com/rune-rs/rune/pull/175
[#176]: https://github.com/rune-rs/rune/pull/176
[#177]: https://github.com/rune-rs/rune/pull/177
[#178]: https://github.com/rune-rs/rune/pull/178
[#180]: https://github.com/rune-rs/rune/pull/180
[#181]: https://github.com/rune-rs/rune/pull/181
[#183]: https://github.com/rune-rs/rune/pull/183
[#184]: https://github.com/rune-rs/rune/pull/184
[#185]: https://github.com/rune-rs/rune/pull/185
[#186]: https://github.com/rune-rs/rune/pull/186
[#187]: https://github.com/rune-rs/rune/pull/187
[#188]: https://github.com/rune-rs/rune/pull/188
[#189]: https://github.com/rune-rs/rune/pull/189
[#190]: https://github.com/rune-rs/rune/pull/190
[#191]: https://github.com/rune-rs/rune/pull/191
[#192]: https://github.com/rune-rs/rune/pull/192
[#193]: https://github.com/rune-rs/rune/pull/193
[#194]: https://github.com/rune-rs/rune/pull/194
[#195]: https://github.com/rune-rs/rune/pull/195
[#196]: https://github.com/rune-rs/rune/pull/196
[#197]: https://github.com/rune-rs/rune/pull/197
[#198]: https://github.com/rune-rs/rune/pull/198
[#199]: https://github.com/rune-rs/rune/pull/199
[#201]: https://github.com/rune-rs/rune/pull/201
[#202]: https://github.com/rune-rs/rune/pull/202
[#203]: https://github.com/rune-rs/rune/pull/203
[#204]: https://github.com/rune-rs/rune/pull/204
[#205]: https://github.com/rune-rs/rune/pull/205
[#206]: https://github.com/rune-rs/rune/pull/206
[#208]: https://github.com/rune-rs/rune/pull/208
[#209]: https://github.com/rune-rs/rune/pull/209
[#210]: https://github.com/rune-rs/rune/pull/210
[#211]: https://github.com/rune-rs/rune/pull/211
[#213]: https://github.com/rune-rs/rune/pull/213
[#214]: https://github.com/rune-rs/rune/pull/214
[#215]: https://github.com/rune-rs/rune/pull/215
[#216]: https://github.com/rune-rs/rune/pull/216
[#217]: https://github.com/rune-rs/rune/pull/217
[#218]: https://github.com/rune-rs/rune/pull/218
[#219]: https://github.com/rune-rs/rune/pull/219
[#220]: https://github.com/rune-rs/rune/pull/220
[#221]: https://github.com/rune-rs/rune/pull/221
[#222]: https://github.com/rune-rs/rune/pull/222
[#223]: https://github.com/rune-rs/rune/pull/223
[#227]: https://github.com/rune-rs/rune/pull/227
[#228]: https://github.com/rune-rs/rune/pull/228
[#229]: https://github.com/rune-rs/rune/pull/229
[#230]: https://github.com/rune-rs/rune/pull/230
[#231]: https://github.com/rune-rs/rune/pull/231
[#232]: https://github.com/rune-rs/rune/pull/232
[#233]: https://github.com/rune-rs/rune/pull/233
[#234]: https://github.com/rune-rs/rune/pull/234
[#235]: https://github.com/rune-rs/rune/pull/235
[#239]: https://github.com/rune-rs/rune/pull/239
[#240]: https://github.com/rune-rs/rune/pull/240
[#241]: https://github.com/rune-rs/rune/pull/241
[#242]: https://github.com/rune-rs/rune/pull/242
[#243]: https://github.com/rune-rs/rune/pull/243
[#245]: https://github.com/rune-rs/rune/pull/245
[#246]: https://github.com/rune-rs/rune/pull/246
[#247]: https://github.com/rune-rs/rune/pull/247
[#248]: https://github.com/rune-rs/rune/pull/248

[robojumper]: https://github.com/robojumper
[tgolsson]: https://github.com/tgolsson
[genusistimelord]: https://github.com/genusistimelord
[maxmcd]: https://github.com/maxmcd
[hvithrafn]: https://github.com/hvithrafn

[0.8.0]: https://github.com/rune-rs/rune/compare/0.7.0...0.8.0

[OxidizeBot]: https://github.com/udoprog/OxidizeBot
[AoC]: https://adventofcode.com/

## [0.7.0]

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

[0.7.0]: https://github.com/rune-rs/rune/compare/0.6.16...0.7.0

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
