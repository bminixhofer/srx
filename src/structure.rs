use std::io::Read;

use serde::Deserialize;

// TODO: errors
pub fn string_to_bool(string: &str) -> bool {
    match string {
        "yes" => true,
        "no" => false,
        _ => panic!("unexpected value {}", string),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct SRX {
    pub version: Option<String>,
    pub header: Header,
    pub body: Body,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Header {
    pub segmentsubflows: Option<String>,
    pub cascade: String,
    #[serde(rename = "formathandle")]
    pub handles: Vec<FormatHandle>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormatHandle {
    // 'type' is a keyword
    #[serde(rename = "type")]
    pub kind: String,
    pub include: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    pub languagerules: LanguageRules,
    pub maprules: MapRules,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageRules {
    #[serde(rename = "languagerule")]
    pub rules: Vec<LanguageRule>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapRules {
    #[serde(rename = "languagemap")]
    pub maps: Vec<LanguageMap>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageRule {
    #[serde(rename = "languagerulename")]
    pub name: String,
    #[serde(rename = "rule")]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    // 'break' is a keyword
    #[serde(rename = "break")]
    pub do_break: String,
    pub beforebreak: Option<String>,
    pub afterbreak: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageMap {
    #[serde(rename = "languagepattern")]
    pub pattern: String,
    #[serde(rename = "languagerulename")]
    pub name: String,
}

pub fn from_reader<R: Read>(reader: R) -> SRX {
    serde_xml_rs::from_reader(reader).unwrap()
}

pub fn from_str<S: AsRef<str>>(string: S) -> SRX {
    serde_xml_rs::from_str(string.as_ref()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_example() {
        let srx = from_str(&fs::read_to_string("data/example.xml").unwrap());
        println!("{:#?}", srx);
    }
}
