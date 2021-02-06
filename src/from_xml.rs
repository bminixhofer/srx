use std::{collections::HashMap, convert::TryFrom, io::Read, str::FromStr};

use super::{utils, Language, LanguageRegex, Rule, SRX};
use regex::Regex;
use thiserror::Error;

pub fn string_to_bool(string: &str) -> Result<bool, Error> {
    match string {
        "yes" => Ok(true),
        "no" => Ok(false),
        x => Err(Error::SRXError {
            reason: format!("unexpected boolean value '{}'. Expected 'yes' or 'no'.", x),
        }),
    }
}

#[derive(Debug, Error)]
#[cfg(feature = "from_xml")]
pub enum Error {
    #[error("Error constructing regex: {0}")]
    RegexError(#[from] regex::Error),
    #[error("Error reading XML: {0}")]
    XMLError(#[from] serde_xml_rs::Error),
    #[error("invalid SRX: {reason}")]
    SRXError { reason: String },
}

impl Rule {
    /// Creates a new ruel.
    ///
    /// # Errors
    ///
    /// If neither `before_break` nor `after_break` is set.
    pub fn new<S1: AsRef<str>, S2: AsRef<str>>(
        before_break: Option<S1>,
        after_break: Option<S2>,
        do_break: bool,
    ) -> Result<Self, Error> {
        if before_break.is_none() && after_break.is_none() {
            return Err(Error::SRXError {
                reason: "either `before_break` or `after_break` must be set".into(),
            });
        }

        Ok(Rule {
            regex: Regex::new(&format!(
                "{}({})",
                before_break.as_ref().map_or("", |x| x.as_ref()),
                after_break.as_ref().map_or("", |x| x.as_ref())
            ))?,
            do_break,
        })
    }
}

impl SRX {
    /// Creates a new SRX struct from a reader.
    ///
    /// # Errors
    ///
    /// * If the file is not in valid SRX format.
    /// * If an unsupported rule is encountered in the `<maprules>`.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, Error> {
        schema::from_reader(reader)
            .map_err(Error::from)
            .and_then(SRX::try_from)
    }
}

impl FromStr for SRX {
    type Err = Error;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        schema::from_str(string)
            .map_err(Error::from)
            .and_then(SRX::try_from)
    }
}

impl TryFrom<schema::SRX> for SRX {
    type Error = Error;

    fn try_from(data: schema::SRX) -> Result<Self, Self::Error> {
        let cascade = string_to_bool(&data.header.cascade)?;

        let map: Result<Vec<_>, Error> = data
            .body
            .maprules
            .maps
            .into_iter()
            .map(|lang| {
                Ok(LanguageRegex {
                    regex: utils::full_regex(&lang.pattern)?,
                    language: Language(lang.name),
                })
            })
            .collect();
        let map = map?;

        let mut errors: HashMap<_, _> = data
            .body
            .languagerules
            .rules
            .iter()
            .map(|lang| (Language(lang.name.clone()), Vec::new()))
            .collect();

        let rules: Result<HashMap<_, _>, Error> = data
            .body
            .languagerules
            .rules
            .into_iter()
            .map(|lang| {
                let key = Language(lang.name);

                let value: Vec<_> = lang
                    .rules
                    .into_iter()
                    .map(|rule| {
                        Ok((
                            rule.beforebreak,
                            rule.afterbreak,
                            string_to_bool(&rule.do_break)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, Error>>()?
                    .into_iter()
                    .filter_map(|(before_break, after_break, do_break)| {
                        let rule = Rule::new(before_break, after_break, do_break);

                        match rule {
                            Ok(rule) => Some(rule),
                            Err(error) => {
                                errors
                                    .get_mut(&key)
                                    .expect("error map has a key for each language")
                                    .push(format!("{}", error));
                                None
                            }
                        }
                    })
                    .collect();

                Ok((key, value))
            })
            .collect();
        let rules = rules?;

        if let Some(entry) = map
            .iter()
            .find(|entry| !rules.iter().any(|rule| *rule.0 == entry.language))
        {
            return Err(Error::SRXError { reason: format!("<languagerules> must have an entry for each language in <languagemap>. Did not find entry for {}", entry.language.0)});
        }

        Ok(SRX {
            cascade,
            map,
            rules,
            errors,
        })
    }
}

mod schema {
    use serde::Deserialize;
    use std::io::Read;

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "serde_crate", rename_all = "lowercase")]
    pub struct SRX {
        pub version: Option<String>,
        pub header: Header,
        pub body: Body,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "serde_crate")]
    pub struct Header {
        pub segmentsubflows: Option<String>,
        pub cascade: String,
        #[serde(rename = "formathandle")]
        pub handles: Vec<FormatHandle>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "serde_crate", deny_unknown_fields)]
    pub struct FormatHandle {
        // 'type' is a keyword
        #[serde(rename = "type")]
        pub kind: String,
        pub include: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "serde_crate", deny_unknown_fields)]
    pub struct Body {
        pub languagerules: LanguageRules,
        pub maprules: MapRules,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "serde_crate", deny_unknown_fields)]
    pub struct LanguageRules {
        #[serde(rename = "languagerule")]
        pub rules: Vec<LanguageRule>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "serde_crate", deny_unknown_fields)]
    pub struct MapRules {
        #[serde(rename = "languagemap")]
        pub maps: Vec<LanguageMap>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "serde_crate", deny_unknown_fields)]
    pub struct LanguageRule {
        #[serde(rename = "languagerulename")]
        pub name: String,
        #[serde(rename = "rule")]
        pub rules: Vec<Rule>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "serde_crate", deny_unknown_fields)]
    pub struct Rule {
        // 'break' is a keyword
        #[serde(rename = "break")]
        pub do_break: String,
        pub beforebreak: Option<String>,
        pub afterbreak: Option<String>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "serde_crate", deny_unknown_fields)]
    pub struct LanguageMap {
        #[serde(rename = "languagepattern")]
        pub pattern: String,
        #[serde(rename = "languagerulename")]
        pub name: String,
    }

    pub fn from_reader<R: Read>(reader: R) -> Result<SRX, serde_xml_rs::Error> {
        serde_xml_rs::from_reader(reader)
    }

    pub fn from_str<S: AsRef<str>>(string: S) -> Result<SRX, serde_xml_rs::Error> {
        serde_xml_rs::from_str(string.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io};

    #[test]
    fn load_example_schema() -> Result<(), io::Error> {
        let srx = schema::from_str(&fs::read_to_string("data/example.srx")?);
        assert!(srx.is_ok());

        Ok(())
    }

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
    fn serde_works() -> Result<(), bincode::Error> {
        let srx =
            SRX::from_str(&fs::read_to_string("data/example.srx").expect("example file exists"))
                .expect("example file is valid");

        let buf = bincode::serialize(&srx)?;
        let deserialized_srx: SRX = bincode::deserialize(&buf)?;

        // we can't actually compare them but some basic comparison of fields is enough
        assert_eq!(srx.map.len(), deserialized_srx.map.len());
        assert_eq!(srx.rules.len(), deserialized_srx.rules.len());
        assert_eq!(srx.cascade, deserialized_srx.cascade);

        Ok(())
    }
}
