use std::{collections::HashMap, convert::TryFrom, io::Read, str::FromStr};
use thiserror::Error;

use regex::Regex;

mod structure;
mod utils;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Language(pub String);

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error constructing regex: {0}")]
    RegexError(#[from] regex::Error),
    #[error("Error reading XML: {0}")]
    XMLError(#[from] serde_xml_rs::Error),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
struct Rule {
    regex: Regex,
    do_break: bool,
}

impl Rule {
    fn new<S1: AsRef<str>, S2: AsRef<str>>(
        before_break: Option<S1>,
        after_break: Option<S2>,
        do_break: bool,
    ) -> Result<Self, regex::Error> {
        assert!(
            before_break.is_some() || after_break.is_some(),
            "either `before_break` or `after_break` must be set"
        );

        Ok(Rule {
            regex: Regex::new(&format!(
                "{}({})",
                before_break.as_ref().map_or("", |x| x.as_ref()),
                after_break.as_ref().map_or("", |x| x.as_ref())
            ))?,
            do_break,
        })
    }

    fn match_indices<'a>(&'a self, text: &'a str) -> impl Iterator<Item = usize> + 'a {
        self.regex.captures_iter(text).map(|x| {
            x.get(1)
                .expect("rule regex must have one capture group denoting the `after_break` part")
                .start()
        })
    }

    fn do_break(&self) -> bool {
        self.do_break
    }
}

#[derive(Debug, Clone, Default)]
pub struct Rules {
    rules: Vec<Rule>,
}

impl Rules {
    pub fn split<'a>(&self, text: &'a str) -> Vec<&'a str> {
        let mut segments = Vec::new();

        let mut mask: Vec<Option<bool>> = vec![None; text.len()];

        for rule in &self.rules {
            for index in rule.match_indices(text) {
                if mask[index].is_none() {
                    mask[index] = Some(rule.do_break());
                }
            }
        }

        let mut prev_index = 0;

        for (i, mask_val) in (0..text.len()).zip(mask) {
            if let Some(true) = mask_val {
                segments.push(&text[prev_index..i]);
                prev_index = i;
            }
        }

        if prev_index != text.len() {
            segments.push(&text[prev_index..]);
        }

        segments
    }
}

#[derive(Debug, Clone)]
pub struct SRX {
    cascade: bool,
    map: Vec<(Regex, Language)>,
    rules: HashMap<Language, Vec<Rule>>,
    errors: HashMap<Language, Vec<regex::Error>>,
}

impl SRX {
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, Error> {
        structure::from_reader(reader)
            .map_err(Error::from)
            .and_then(SRX::try_from)
    }

    pub fn language_rules(&self, lang_code: &str) -> Rules {
        let mut rules = Vec::new();

        for (regex, language) in &self.map {
            if regex.is_match(lang_code) {
                rules.extend(self.rules.get(language).expect("languagerulename in <languagemap> must have a corresponding entry in <languagerules>"));
                if !self.cascade {
                    break;
                }
            }
        }

        Rules {
            rules: rules.into_iter().cloned().collect(),
        }
    }

    pub fn errors(&self) -> &HashMap<Language, Vec<regex::Error>> {
        &self.errors
    }
}

impl FromStr for SRX {
    type Err = Error;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        structure::from_str(string)
            .map_err(Error::from)
            .and_then(SRX::try_from)
    }
}

impl TryFrom<structure::SRX> for SRX {
    type Error = Error;

    fn try_from(data: structure::SRX) -> Result<Self, Self::Error> {
        let cascade = structure::string_to_bool(&data.header.cascade);

        let map: Result<Vec<_>, Error> = data
            .body
            .maprules
            .maps
            .into_iter()
            .map(|lang| Ok((utils::full_regex(&lang.pattern)?, Language(lang.name))))
            .collect();
        let map = map?;

        let mut errors: HashMap<_, _> = data
            .body
            .languagerules
            .rules
            .iter()
            .map(|lang| (Language(lang.name.clone()), Vec::new()))
            .collect();

        let rules: HashMap<_, _> = data
            .body
            .languagerules
            .rules
            .into_iter()
            .map(|lang| {
                let key = Language(lang.name);
                let value: Vec<_> = lang
                    .rules
                    .into_iter()
                    .filter_map(|rule| {
                        let rule = Rule::new(
                            rule.beforebreak,
                            rule.afterbreak,
                            structure::string_to_bool(&rule.do_break),
                        );

                        match rule {
                            Ok(rule) => Some(rule),
                            Err(error) => {
                                errors
                                    .get_mut(&key)
                                    .expect("error map has a key for each language")
                                    .push(error);
                                None
                            }
                        }
                    })
                    .collect();

                (key, value)
            })
            .collect();

        Ok(SRX {
            cascade,
            map,
            rules,
            errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn getting_language_rules_works() {
        let srx =
            SRX::from_str(&fs::read_to_string("data/example.srx").expect("example file exists"))
                .expect("example file is valid");

        assert!(!srx.language_rules("en").rules.is_empty());
        assert_ne!(
            srx.language_rules("en").rules.len(),
            srx.language_rules("fr").rules.len()
        );
    }

    #[test]
    fn match_indices_correct() {
        let rule = Rule::new(Some("abc"), Some("d+fg"), true).expect("test rule is valid");

        assert_eq!(rule.match_indices("abcddfg").collect::<Vec<_>>(), vec![3]);
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
            rules.split(text),
            vec![
                "The U.K. Prime Minister, Mr. Blair, was seen out with his family today.",
                " He is well."
            ]
        );
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
