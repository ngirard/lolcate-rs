use toml::de::Error;
use std::{convert, fs, io::prelude::*, path};
use std::collections::HashMap;
use toml::macros::Deserialize;
use std::process;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub description: String,
    pub dirs: Vec<path::PathBuf>,
    pub include_dirs: bool,
    pub ignore_symlinks: bool,
    pub ignore_hidden: bool,
}

#[derive(Debug, Deserialize)]
pub struct GlobalConfig {
    pub types: HashMap<String, String>,
}


pub fn read_toml_file<'a, 'de, P: ?Sized, T>(path: &'a P, buffer: &'de mut String) -> Result<T, Error>
where
    P: convert::AsRef<path::Path>,
    T: Deserialize<'de>,
{
    let mut configuration_file: fs::File 
        = match fs::OpenOptions::new()
          .read(true)
          .open(path) {
             Ok(val) => val,
             Err(_e) => {
                eprintln!("Cannot open file {}", path.as_ref().display());
                process::exit(1);
                }};
                
    match configuration_file.read_to_string(buffer) {
        Ok(_bytes) => {
            toml::from_str(buffer.as_str())
        }
        Err(error) => panic!(
            "
                The data in this stream is not valid UTF-8.\n
                See error: '{}'
                ",
            error
        ),
    }
}
