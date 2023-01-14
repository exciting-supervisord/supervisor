use super::config_error::*;

#[derive(Debug, PartialEq)]
pub struct GeneralConfig {
    pub sockfile: String,
}

impl GeneralConfig {
    pub fn new() -> Self {
        GeneralConfig {
            sockfile: "/tmp/taskmasterd.sock".to_owned(),
        }
    }

    pub fn from(prop: &ini::Properties) -> Result<Self, ConfigKeyError> {
        let mut config = GeneralConfig::new();
        for (k, v) in prop.iter() {
            match k {
                "sockfile" => config.sockfile = v.to_owned(),
                _ => return Err(ConfigKeyError::new(k)),
            }
        }
        Ok(config)
    }
}
