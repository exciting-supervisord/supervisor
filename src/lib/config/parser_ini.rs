extern crate ini;
use ini::Ini;
use std::error::Error;

pub fn load_ini(file_path: &str) -> Result<Ini, Box<dyn Error>> {
    let mut i = Ini::load_from_file(file_path)?;
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
    fn test_load_ini() -> Result<(), Box<dyn Error>> {
        let ini = load_ini("conf.ini")?;
        print!("{ini}");
        Ok(())
    }
}
