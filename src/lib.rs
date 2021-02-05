use std::{collections::HashMap, iter};

use regex::Regex;

mod structure;
mod utils;

#[derive(Debug, Clone)]
#[non_exhaustive]
struct Rule {
    before_break: Option<Regex>,
    after_break: Option<Regex>,
    do_break: bool,
}

impl Rule {
    fn new(before_break: Option<String>, after_break: Option<String>, do_break: bool) -> Self {
        assert!(
            before_break.is_some() || after_break.is_some(),
            "either `before_break` or `after_break` must be set."
        );

        Rule {
            before_break: before_break.map(|x| utils::end_regex(&x).unwrap()),
            after_break: after_break.map(|x| utils::start_regex(&x).unwrap()),
            do_break,
        }
    }

    fn is_match(&self, before: &str, after: &str) -> Option<bool> {
        if self
            .before_break
            .as_ref()
            .map_or(true, |re| re.is_match(before))
            && self
                .after_break
                .as_ref()
                .map_or(true, |re| re.is_match(after))
        {
            Some(self.do_break)
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct SRX {
    cascade: bool,
    map: Vec<(Regex, String)>,
    rules: HashMap<String, Vec<Rule>>,
}

impl From<structure::SRX> for SRX {
    fn from(data: structure::SRX) -> Self {
        let cascade = structure::string_to_bool(&data.header.cascade);
        let map: Vec<_> = data
            .body
            .maprules
            .maps
            .into_iter()
            .map(|lang| (utils::full_regex(&lang.pattern).unwrap(), lang.name))
            .collect();
        let rules: HashMap<_, _> = data
            .body
            .languagerules
            .rules
            .into_iter()
            .map(|lang| {
                let key = lang.name;
                let value: Vec<_> = lang
                    .rules
                    .into_iter()
                    .map(|rule| {
                        Rule::new(
                            rule.beforebreak,
                            rule.afterbreak,
                            structure::string_to_bool(&rule.do_break),
                        )
                    })
                    .collect();

                (key, value)
            })
            .collect();

        SRX {
            cascade,
            map,
            rules,
        }
    }
}

impl SRX {
    fn language(&self, lang_code: &str) -> Rules {
        let mut rules = Vec::new();

        for (regex, language) in &self.map {
            if regex.is_match(lang_code) {
                rules.extend(self.rules.get(language).unwrap());
                if !self.cascade {
                    break;
                }
            }
        }

        Rules {
            rules: rules.into_iter().cloned().collect(),
        }
    }
}

#[derive(Debug)]
struct Rules {
    rules: Vec<Rule>,
}

impl Rules {
    fn split<'a>(&self, text: &'a str) -> Vec<&'a str> {
        let mut segments = Vec::new();
        let mut prev_index = 0;

        for i in text
            .char_indices()
            .map(|(i, _)| i)
            .chain(iter::once(text.len()))
        {
            let matched = self
                .rules
                .iter()
                .find_map(|x| x.is_match(&text[..i], &text[i..]));
            if matches!(matched, Some(true)) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn getting_language_rules_works() {
        let srx: SRX = structure::from_str(&fs::read_to_string("data/example.xml").unwrap()).into();

        println!(
            "{:?}",
            srx.language("en")
                .split("Hello! Hello, I'm in the U.K. currently. Where are you?")
        );
    }
}
