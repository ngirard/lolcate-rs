extern crate dirs;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate lz4;

use std::path::PathBuf;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::process;

mod config;
use crate::config::read_toml_file;
use crate::config::Config;
use walkdir::WalkDir;
use lz4::{EncoderBuilder};

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
            eprintln!("Invalid TOML: {}", error);
            process::exit(1);
        }
    };
    if config.dirs.len() == 0 {
        eprintln!("Please edit file {:?} and add at least a directory to scan.", default_config_file);
        process::exit(1);
    }
    for dir in &config.dirs {
        if !dir.exists() {
            eprintln!("The specified dir {:?} doesn't exist.", dir);
            process::exit(1);
        }
        if !dir.is_dir() {
            eprintln!("The specified path {:?} is not a directory or cannot be accessed.", dir);
            process::exit(1);
        }
    }
    
    let mut db_fn = lolcate_path();
    db_fn.push("db.lz4");
    let output_file = File::create(db_fn)?;
    let mut encoder = EncoderBuilder::new()
        .level(4)
        .build(output_file)?;
    
    for dir in &config.dirs {
        for entry in WalkDir::new(dir.to_str().unwrap()) {
            let entry = entry.unwrap();
            //encoder.write(entry.path().to_str().unwrap().as_bytes())?;
            writeln!(encoder, "{}", entry.path().to_str().unwrap())?;
        }
    }
    
    let (_output, result) = encoder.finish();
    result
    //Ok(())
}
