extern crate dirs;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate lz4;

use std::path::PathBuf;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io;
use std::io::prelude::*;
use std::process;
use std::str;

mod config;
mod cli;
use crate::config::read_toml_file;
use crate::config::Config;
use walkdir::WalkDir;
use lz4::{EncoderBuilder};


//#[macro_use] extern crate lazy_static;
extern crate regex;

use regex::Regex;



static DEFAULT_PROJECT_TEMPLATE : &str = r#"
# Dirs to add
dirs = [
  # "/first/dir",
  # "/second/dir"
]

[ignore]
# Files / dirs to ignore

"#;


pub fn lolcate_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap();
    path.push("lolcate-rs");
    path
}

fn get_config(toml_file: &PathBuf) -> Config {
    let mut buffer = String::new();
    let config: Config = match read_toml_file(&toml_file, &mut buffer) {
        Ok(config) => {
            config
        }
        Err(error) => {
            eprintln!("Invalid TOML: {}", error);
            process::exit(1);
        }
    };
    
    // Check config
    if config.dirs.len() == 0 {
        eprintln!("Please edit file {:?} and add at least a directory to scan.", toml_file);
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
    config
}

fn db_fn(db_name: &str) -> PathBuf {
    let mut db_fn = lolcate_path();
    db_fn.push(db_name);
    db_fn.push("db.lz4");
    db_fn
}

fn update_database(database: &str) -> std::io::Result<()> {
    let mut config_fn = lolcate_path();
    config_fn.push(database);
    config_fn.push("config.toml");
    if !config_fn.exists() {
        eprintln!("Config file not found for database {}.\nPerhaps you forgot to type lolcate --create {} ?", &database, &database);
        process::exit(1);
    }
    let config = get_config(&config_fn);
    
    let output_fn = File::create(db_fn(&database))?;
    let mut encoder = EncoderBuilder::new()
        .level(4)
        .build(output_fn)?;
    
    println!("Updating {}...", database);
    for dir in &config.dirs {
        for entry in WalkDir::new(dir.to_str().unwrap()) {
            let entry = entry.unwrap();
            //encoder.write(entry.path().to_str().unwrap().as_bytes())?;
            writeln!(encoder, "{}", entry.path().to_str().unwrap())?;
        }
    }
    
    let (_output, result) = encoder.finish();
    result
}

fn db_lookup(database: &str, pattern: &str) -> std::io::Result<()> {
    //println!("Lookup {} for {}", &database, &pattern);
    let input_file = File::open(db_fn(&database))?;
    let decoder = lz4::Decoder::new(input_file)?;
    let reader = io::BufReader::new(decoder);
    
    //lazy_static! {
        //static ref 
    let re: Regex = Regex::new(pattern).unwrap();
    //}

    
    for _line in reader.lines() {
        let line = _line.unwrap();
        if re.is_match(&line) {
            println!("{}", &line);
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {

    // 1. Parse command-line args
    let app = cli::build_cli();
    let args = app.get_matches();
    
    let database = args.value_of("database").unwrap();
    
    if args.is_present("update") {
        update_database(&database)?;
        process::exit(0);
    }
    
    if let Some(pattern) = args.value_of("pattern") {
        //println!("Lookup: {}", pattern);
        db_lookup(&database, &pattern)?;
        process::exit(0);
    }

    let mut default_config_fn = lolcate_path();
    default_config_fn.push("config.toml");
    if !default_config_fn.exists() {
        fs::create_dir_all(default_config_fn.parent().unwrap())?;
        let mut f = File::create(&default_config_fn)?;
        f.write_all(DEFAULT_PROJECT_TEMPLATE.as_bytes())?;
        println!("Added default database.\nPlease edit file {:?}.", default_config_fn);
        process::exit(0);
    }
    
    Ok(())
}
