use regex::{self, Regex};

pub fn full_regex<S: AsRef<str>>(re: S) -> Result<Regex, regex::Error> {
    let pattern = format!("^{}$", re.as_ref());

    Regex::new(&pattern)
}
