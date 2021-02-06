# SRX

[![Crates.io](https://img.shields.io/crates/v/srx)](https://crates.io/crates/srx)
[![Docs.rs](https://docs.rs/srx/badge.svg)](https://docs.rs/srx)
![MIT OR Apache 2.0 license](https://img.shields.io/crates/l/srx)

A simple and reasonably fast Rust implementation of the [Segmentation Rules eXchange 2.0 standard](https://www.unicode.org/uli/pas/srx/srx20.html) for text segmentation. `srx` is *not* fully compliant with the standard.

This crate is intended for segmentation of plaintext so markup information (`<formathandle>` and `segmentsubflows`) is ignored.

Not complying with the SRX spec, overlapping matches of the same `<rule>` are not found which could lead to different behavior in a few edge cases.

## A note on regular expressions

This crate uses the [`regex` crate](https://github.com/rust-lang/regex) for parsing and executing regular expressions. The `regex` crate is mostly compatible with the [regular expression standard](https://www.unicode.org/uli/pas/srx/srx20.html#Intro_RegExp) from the SRX specification. However, some metacharacters such as `\Q` and `\E` are not supported.

To still be able to use files containing unsupported rules and to parse useful SRX files such as [`segment.srx` from LanguageTool](https://github.com/languagetool-org/languagetool/blob/master/languagetool-core/src/main/resources/org/languagetool/resource/segment.srx) which does not comply with the standard by e. g. using look-ahead and look-behind, `srx` ignores `<rule>` elements with invalid regular expressions and provides information about them via the `srx.errors()` function.