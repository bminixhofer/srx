use regex::{self, Regex};

pub fn end_regex<S: AsRef<str>>(re: S) -> Result<Regex, regex::Error> {
    Regex::new(&format!("{}$", re.as_ref()))
}

pub fn start_regex<S: AsRef<str>>(re: S) -> Result<Regex, regex::Error> {
    Regex::new(&format!("^{}", re.as_ref()))
}

pub fn full_regex<S: AsRef<str>>(re: S) -> Result<Regex, regex::Error> {
    Regex::new(&format!("^{}$", re.as_ref()))
}
