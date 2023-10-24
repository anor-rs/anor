/// Build Profile
/// https://doc.rust-lang.org/cargo/reference/profiles.html
#[derive(Debug, PartialEq)]
pub enum BuildProfile {
    /// cargo build --release
    Release,

    /// cargo build
    Dev,

    /// cargo test
    Test,
    // Bench,
    // Doc,
}

/// Returns whether debug information is included in the build
pub fn debug_mode() -> bool {
    #[cfg(debug_assertions)]
    return true;

    #[cfg(not(debug_assertions))]
    return false;
}

/// Returns whether the build profile is test
pub fn is_cargo_test() -> bool {
    build_profile() == BuildProfile::Test
}

/// Returns a `BuildProfile` value according to the build
pub fn build_profile() -> BuildProfile {
    if cfg!(test) {
        return BuildProfile::Test;
    }
    if debug_mode() {
        return BuildProfile::Dev;
    }

    BuildProfile::Release
}

#[cfg(test)]
mod test {
    use crate::utils::build_profile::*;

    #[test]
    fn build_profile_test() {
        assert_eq!(build_profile(), BuildProfile::Test);
    }

    #[test]
    fn is_build_profile_test_() {
        assert!(is_cargo_test());
    }

    #[test]
    fn debug_mode_test() {
        assert!(debug_mode());
    }
}
