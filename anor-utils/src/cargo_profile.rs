/// Cargo build profile
/// <https://doc.rust-lang.org/cargo/reference/profiles.html>
#[derive(Debug, PartialEq)]
pub enum CargoProfile {
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
pub fn is_profile_test() -> bool {
    get_profile() == CargoProfile::Test
}

/// Returns a `BuildProfile` value according to the cargo command
pub fn get_profile() -> CargoProfile {
    if cfg!(test) {
        return CargoProfile::Test;
    }
    if debug_mode() {
        return CargoProfile::Dev;
    }

    CargoProfile::Release
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get_profile_test() {
        assert_eq!(get_profile(), CargoProfile::Test);
    }

    #[test]
    fn is_cargo_profile_test() {
        assert!(is_profile_test());
    }

    #[test]
    fn debug_mode_test() {
        assert!(debug_mode());
    }
}
