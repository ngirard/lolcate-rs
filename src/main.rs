extern crate dirs;
#[macro_use]
extern crate serde_derive;
extern crate serde;

use std::path::PathBuf;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::process;

mod config;
use crate::config::read_toml_file;
use crate::config::Config;

static DEFAULT_PROJECT_TEMPLATE : &str = r#"
# Dirs to add
dirs = [
  # "/first/dir",
  # "/second/DIR"
]

[ignore]
# Files / dirs to ignore

"#;


pub fn lolcate_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap();
    path.push("lolcate-rs");
    path
}

fn main() -> std::io::Result<()> {
    let mut default_config_file = lolcate_path();
    default_config_file.push("config.toml");
    if !default_config_file.exists() {
        fs::create_dir_all(default_config_file.parent().unwrap())?;
        let mut f = File::create(&default_config_file)?;
        f.write_all(DEFAULT_PROJECT_TEMPLATE.as_bytes())?;
        println!("Added default database.\nPlease edit file {:?}.", default_config_file);
        process::exit(0);
    }
    
    let mut buffer = String::new();
    let config: Config = match read_toml_file(&default_config_file, &mut buffer) {
        Ok(config) => {
            config
        }
        Err(error) => {
            println!("Invalid TOML: {}", error);
            process::exit(1 );
        }
    };
    if config.dirs.len() == 0 {
        println!("Please edit file {:?} and add at least a directory to scan.", default_config_file);
        process::exit(1);
    }
    Ok(())
}
