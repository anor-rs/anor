pub fn substitute(src: &str) -> String {
    src.to_string()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn subst1() {
        let src = "${CARGO_MANIFEST_DIR}/target/tmp/anor";
        let found = substitute(src);

        let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join("anor");

        assert_eq!(found, expected.to_string_lossy());
    }
}
