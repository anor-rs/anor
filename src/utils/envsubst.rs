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

    #[test]
    fn substitute_string_test() {
        let src = "**${CARGO_PKG_NAME}**";
        assert_eq!(substitute(src), "**anor-common**");
    }

    #[test]
    fn substitute_path_test() {
        let src = "${CARGO_MANIFEST_DIR}/1/2/3";
        let expected = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("1")
            .join("2")
            .join("3");

        assert_eq!(substitute(src), expected.to_string_lossy());
    }
}
