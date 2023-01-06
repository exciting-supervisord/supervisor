extern crate ini;
use super::config_error::*;
use ini::Ini;

pub fn load_ini(file_path: &str) -> Result<Ini, ConfigFileError> {
    let mut i = Ini::load_from_file(file_path).map_err(|e| match e {
        ini::Error::Io(_) => ConfigFileError::Nofile(ConfigNoFileError),
        ini::Error::Parse(_) => ConfigFileError::Parsing(ConfigParsingError),
    })?;
    for (_, prop) in i.iter_mut() {
        for (_, v) in prop.iter_mut() {
            *v = v.split(';').collect::<Vec<&str>>()[0].to_owned();
        }
    }
    Ok(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_ini() -> Result<(), ConfigFileError> {
        let ini = load_ini("conf.ini")?;
        print!("{ini}");
        Ok(())
    }
}
