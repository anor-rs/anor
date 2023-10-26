use regex::Regex;

pub fn substitute(src: &str) -> String {
    let mut result = src.to_string();
    let regex = Regex::new(r"\$\{(.*?)\}").unwrap();

    for token in regex.find_iter(src) {
        let key = token.as_str().to_string();
        let env_key = key.replace(['$', '{', '}'], "");
        if let Ok(env_value) = std::env::var(env_key) {
            result = result.replace(&key, &env_value);
        }
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn substitute_test() {
        let src = "${CARGO_MANIFEST_DIR}/target/tmp/anor";
        let found = substitute(src);

        let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join("anor");

        assert_eq!(found, expected.to_string_lossy());
    }
}
