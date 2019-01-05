use toml::de::Error;
use std::{convert, fs, io::prelude::*, path}; // env, io
use toml::macros::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub dirs: Vec<path::PathBuf>,
    pub ignore: Vec<path::PathBuf>,
}

pub fn read_toml_file<'a, 'de, P: ?Sized, T>(path: &'a P, buffer: &'de mut String) -> Result<T, Error>
where
    P: convert::AsRef<path::Path>,
    T: Deserialize<'de>,
{
    let mut configuration_file: fs::File = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .expect("Cannot read the configuration file");
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
