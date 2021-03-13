//! A simple and reasonably fast Rust implementation of the
//! [Segmentation Rules eXchange 2.0 standard](https://www.unicode.org/uli/pas/srx/srx20.html)
//! for text segmentation. `srx` is *not* fully compliant with the standard.
//!
//! This crate is intended for segmentation of plaintext so markup information (`<formathandle>` and `segmentsubflows`)
//! is ignored.
//!
//! Not complying with the SRX spec, overlapping matches of the same `<rule>` are not found which could
//! lead to different behavior in a few edge cases.
//!
//! ## Example
//!
//! ```
//! use std::{fs, str::FromStr};
//! use srx::SRX;
//!
//! let srx = SRX::from_str(&fs::read_to_string("data/segment.srx").unwrap())?;
//! let english_rules = srx.language_rules("en");
//!
//! assert_eq!(
//!     english_rules.split("e.g. U.K. and Mr. do not split. SRX is a rule-based format.").collect::<Vec<_>>(),
//!     vec!["e.g. U.K. and Mr. do not split. ", "SRX is a rule-based format."]
//! );
//! # Ok::<(), srx::Error>(())
//! ```
//!
//! ## Features
//!
//! - `serde`: Serde serialization and deserialization support for [SRX].
//! - `from_xml`: [SRX::from_reader] method and [std::str::FromStr] implementation to load from an XML file in SRX format.
//!
//! ## A note on regular expressions
//!
//! This crate uses the [`regex` crate](https://github.com/rust-lang/regex) for parsing and executing
//! regular expressions. The `regex` crate is mostly compatible with the
//! [regular expression standard](https://www.unicode.org/uli/pas/srx/srx20.html#Intro_RegExp) from the SRX specification.
//! However, some metacharacters such as `\Q` and `\E` are not supported.
//!
//! To still be able to use files containing unsupported rules and to parse useful SRX files
//! such as
//! [`segment.srx` from LanguageTool](https://github.com/languagetool-org/languagetool/blob/master/languagetool-core/src/main/resources/org/languagetool/resource/segment.srx)
//! which does not comply with the standard by e. g. using look-ahead and look-behind, `srx`
//! ignores `<rule>` elements with invalid regular expressions and provides information about
//! them via the [SRX::errors] function.
#![cfg_attr(docsrs, feature(doc_cfg))] // see https://stackoverflow.com/a/61417700
#[cfg(feature = "serde")]
extern crate serde_crate as serde;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, ops::Range};

use regex::Regex;

#[cfg(feature = "from_xml")]
mod from_xml;
#[cfg(feature = "from_xml")]
mod utils;
#[cfg(feature = "from_xml")]
pub use from_xml::Error;

/// Newtype denoting a language (`languagerulename` attribute in SRX).
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Language(pub String);

/// A single SRX rule. In SRX, consists of one `before_break` and one `after_break` Regex.
/// For efficiency this crate compiles these regexes into one regex of the form `before_break(after_break)`
/// and uses the start of the first capture group as the split index.
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[derive(Debug, Clone)]
#[non_exhaustive]
struct Rule {
    #[cfg_attr(feature = "serde", serde(with = "serde_regex"))]
    regex: Regex,
    do_break: bool,
}

impl Rule {
    /// Gets all byte indices in the text at which this rule matches.
    /// Contrary to the SRX 2.0 spec this does not find overlapping matches.
    fn match_indices<'a>(&'a self, text: &'a str) -> impl Iterator<Item = usize> + 'a {
        self.regex.captures_iter(text).filter_map(|x| {
            // generally it is guaranteed that a regex has
            // at least one match, but be lenient about
            // errors in the srx xml files and drop those without
            x.get(1).map(|x| x.start())
        })
    }

    /// Whether this rule breaks or prevents breaking.
    fn do_break(&self) -> bool {
        self.do_break
    }
}

/// An ordered set of rules.
/// Rules are executed in order.
/// Once a rule matches on an index, no other rule can match at the same index.
/// Each rule either breaks (i. e. splits the text at this index) or prevents breaking.
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[derive(Debug, Clone, Default)]
pub struct Rules {
    rules: Vec<Rule>,
}

impl Rules {
    /// Obtain the ranges for text segments. Guaranteed to be at character bounds.
    pub fn split_ranges(&self, text: &str) -> Vec<Range<usize>> {
        let mut segments = Vec::new();

        // TODO use a proper tri-state enum here
        let mut masked_bytes: Vec<Option<bool>> = vec![None; text.len()];

        'outer: for rule in &self.rules {
            for byte_index in rule.match_indices(text) {

                if byte_index >= text.len() {
                    continue 'outer;
                }

                if masked_bytes[byte_index].is_none() {
                    masked_bytes[byte_index] = Some(rule.do_break());
                }
            }
        }

        let mut prev_byte_pos = 0;

        // Iterate over characters, we don't want no half characters in the output ranges
        for (byte_pos, _c) in text.char_indices() {
            if let Some(Some(true)) = masked_bytes.get(byte_pos) {
                segments.push(prev_byte_pos..byte_pos);
                prev_byte_pos = byte_pos;
            }
        }

        // Deal with the trailing element, which is by definition
        // not required to be suffixed by a gap char.
        if text[prev_byte_pos..].chars().next().is_some() {
            segments.push(prev_byte_pos..text.len());
        }

        segments
    }

    /// Split text into sentences.
    pub fn split<'a, 'b>(&self, text: &'a str) -> impl Iterator<Item = &'a str> + 'b
    where
        'a: 'b,
    {
        self.split_ranges(text)
            .into_iter()
            .map(move |range| &text[range])
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }
}

/// An entry of the `<maprules>` element.
/// Associates a regex with a [Language].
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[derive(Debug, Clone)]
struct LanguageRegex {
    #[cfg_attr(feature = "serde", serde(with = "serde_regex"))]
    regex: Regex,
    language: Language,
}

/// The SRX root.
/// Does not execute rules on is own.
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[derive(Debug, Clone)]
pub struct SRX {
    cascade: bool,
    map: Vec<LanguageRegex>,
    rules: HashMap<Language, Vec<Rule>>,
    errors: HashMap<Language, Vec<String>>,
}

impl SRX {
    /// Gets the rules for a language code by
    /// - aggregating rules from all [Language]s with a matching `<languagepattern>` (if the SRX is set to be cascading)
    /// - finding the first matching `<languagepattern>` (if the SRX is set to be not cascading)
    ///
    /// Result should be cached instead of calling this repeatedly as it clones the rules.
    pub fn language_rules<S: AsRef<str>>(&self, lang_code: S) -> Rules {
        let mut rules = Vec::new();

        for item in &self.map {
            if item.regex.is_match(lang_code.as_ref()) {
                rules.extend(self.rules.get(&item.language).expect("languagerulename in <languagemap> must have a corresponding entry in <languagerules>").iter().cloned());
                if !self.cascade {
                    break;
                }
            }
        }

        Rules { rules }
    }

    /// Maps [Language]s to a vector of string representations of errors which occured during parsing regular expressions for this language.
    pub fn errors(&self) -> &HashMap<Language, Vec<String>> {
        &self.errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, str::FromStr};

    #[test]
    fn match_indices_correct() {
        let rule = Rule::new(Some("abc"), Some("d+fg"), true).expect("test rule is valid");

        assert_eq!(
            rule.match_indices("abcddfgxxx").collect::<Vec<_>>(),
            vec![3_usize]
        );
    }

    #[test]
    fn example_splits_correct() {
        let rules =
            SRX::from_str(&fs::read_to_string("data/example.srx").expect("example file exists"))
                .expect("example file is valid")
                .language_rules("en");

        // example from the spec: https://www.unicode.org/uli/pas/srx/srx20.html#AppExample
        let text =
            "The U.K. Prime Minister, Mr. Blair, was seen out with his family today. He is well.";
        assert_eq!(
            rules.split(text).collect::<Vec<_>>(),
            vec![
                "The U.K. Prime Minister, Mr. Blair, was seen out with his family today.",
                " He is well."
            ]
        );
    }
    #[test]
    fn example_splits_correct_multi_emoji() {
        let rules =
            SRX::from_str(&fs::read_to_string("data/segment.srx").expect("example file exists"))
                .expect("example file is valid")
                .language_rules("en");

        let text = "e.g. U.K. and Mr. do not split. SRX is a 👒🍏🍱-based format 🐱";
        assert_eq!(
            rules.split(text).collect::<Vec<_>>(),
            vec![
                "e.g. U.K. and Mr. do not split. ",
                "SRX is a 👒🍏🍱-based format 🐱"
            ]
        );
    }

    #[test]
    fn ignores_last_match_index() {
        let rules =
            SRX::from_str(&fs::read_to_string("data/segment.srx").expect("example file exists"))
                .expect("example file is valid")
                .language_rules("en");

        let _ = rules.split("Hello! ").collect::<Vec<_>>();
    }

    #[test]
    fn errors_reported() {
        let srx =
            SRX::from_str(&fs::read_to_string("data/segment.srx").expect("segment file exists"))
                .expect("segment file is valid");

        assert!(!srx.errors().is_empty());
        assert_eq!(srx.errors().values().flatten().count(), 51);
    }
}
