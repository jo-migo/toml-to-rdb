pub mod environment {
    use regex::Regex;
    use std::env;

    const SEMVAR_REGEX: &str = r"^([0-9]+)(\.[0-9]+)?(\.[0-9]+)?";
    const INVALID_REDIS_VERSION_MSG: &str = "Invalid value for REDIS_VERSION set in env";

    fn get_major_version(semantic_version: &str) -> u8 {
        let semvar_regex = Regex::new(SEMVAR_REGEX).unwrap();
        let major_version = match semvar_regex.captures(semantic_version) {
            Some(version) => version.get(1).expect(INVALID_REDIS_VERSION_MSG).as_str(),
            None => return crate::DEFAULT_REDIS_VERSION,
        };

        major_version
            .parse::<u8>()
            .expect(INVALID_REDIS_VERSION_MSG)
    }

    pub fn get_redis_version() -> u8 {
        let redis_version = match env::var_os("REDIS_VERSION") {
            Some(v) => v.into_string().unwrap().to_string(),
            None => crate::DEFAULT_REDIS_VERSION.to_string(),
        };
        get_major_version(&redis_version)
    }
}
