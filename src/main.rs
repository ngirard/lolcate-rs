extern crate dirs;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate lz4;
extern crate ignore;
extern crate walkdir;

use std::path::PathBuf;
use std::fs;
use std::io::Write;
use std::io;
use std::io::prelude::*;
use std::process;
use std::str;

mod config;
mod cli;
use crate::config::read_toml_file;
use crate::config::Config;
use lz4::{EncoderBuilder};


//#[macro_use] extern crate lazy_static;
extern crate regex;

use regex::Regex;



static PROJECT_CONFIG_TEMPLATE : &str = r#"
description = ""

# Dirs to add.
dirs = [
  # "/first/dir",
  # "/second/dir"
]

# Set to true if you want to index directories
include_dirs = false

# Set to true if you want skip symbolic links
ignore_symlinks = false

# Set to true if you want to index hidden files and directories
ignore_hidden = false
"#;

static PROJECT_IGNORE_TEMPLATE : &str = r#"
# Dirs / files to ignore.
# Use the same syntax as gitignore(5).
# Common patterns:
#
# .git
# *~
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

fn config_fn(db_name: &str) -> PathBuf {
    let mut _fn = lolcate_path();
    _fn.push(db_name);
    _fn.push("config.toml");
    _fn
}

fn db_fn(db_name: &str) -> PathBuf {
    let mut _fn = lolcate_path();
    _fn.push(db_name);
    _fn.push("db.lz4");
    _fn
}

fn ignores_fn(db_name: &str) -> PathBuf {
    let mut _fn = lolcate_path();
    _fn.push(db_name);
    _fn.push("ignores");
    _fn
}

fn create_database(db_name: &str) -> std::io::Result<()> {
    let mut db_dir = lolcate_path();
    db_dir.push(db_name);
    if db_dir.exists() {
        eprintln!("Database {} already exists", &db_name);
        process::exit(1);
    }
    let config_fn = config_fn(&db_name);
    fs::create_dir_all(config_fn.parent().unwrap())?;
    let mut f = fs::File::create(&config_fn)?;
    f.write_all(PROJECT_CONFIG_TEMPLATE.as_bytes())?;
    
    let ignores_fn = ignores_fn(&db_name);
    f = fs::File::create(&ignores_fn)?;
    f.write_all(PROJECT_IGNORE_TEMPLATE.as_bytes())?;
    
    println!("Added database '{}'.\nPlease edit file {:?}.", db_name, config_fn);
    process::exit(0);
}

fn list() -> std::io::Result<()> {
    let mut data: Vec<(String,String)> = Vec::new();
    let walker = walkdir::WalkDir::new(lolcate_path()).min_depth(1).into_iter();
    for entry in walker.filter_entry(|e| e.file_type().is_dir()) {
        if let Some(db_name) = entry.unwrap().file_name().to_str(){
            let config_fn = config_fn(&db_name);
            let config = get_config(&config_fn);
            let description = config.description;
            data.push((db_name.to_string(), description.to_string()));
        }
    }
    for (name, desc) in data {
        println!("{:width$}\t{}", name, desc, width=10);
    }
    Ok(())
}

pub fn walker(config: &Config, database: &str) -> ignore::Walk {
    let paths = &config.dirs;
    let mut wd = ignore::WalkBuilder::new(&paths[0]);
    wd.hidden(config.ignore_hidden) // Whether to ignore hidden files
      .parents(false)     // Don't read ignore files from parent directories
      .follow_links(true) // Follow symbolic links
      .ignore(true)       // Don't read .ignore files
      .git_global(false)  // Don't read global gitignore file
      .git_ignore(false)  // Don't read .gitignore files
      .git_exclude(false);// Don't read .git/info/exclude files
        
    for path in &paths[1..] {
        wd.add(path);
    }
    wd.add_ignore(ignores_fn(&database));    
    wd.build()
}

fn update_database(database: &str) -> std::io::Result<()> {
    let config_fn = config_fn(&database);
    if !config_fn.exists() {
        eprintln!("Config file not found for database {}.\nPerhaps you forgot to type lolcate --create {} ?", &database, &database);
        process::exit(1);
    }
    let config = get_config(&config_fn);
    let include_dirs = config.include_dirs;
    let ignore_symlinks = config.ignore_symlinks;
    
    let output_fn = fs::File::create(db_fn(&database))?;
    let mut encoder = EncoderBuilder::new()
        .level(4)
        .build(output_fn)?;
    
    println!("Updating {}...", database);
    for entry in walker(&config, &database) {
        let entry = entry.unwrap();
        if !include_dirs || ignore_symlinks {
            if let Some(ft) = entry.file_type() {
                if !include_dirs && ft.is_dir() {
                    continue;
                }
                if ignore_symlinks && ft.is_symlink() {
                    continue;
                }
            } else {
                continue; // entry is stdin
            }
        }
        writeln!(encoder, "{}", entry.path().to_str().unwrap())?;
    }
    
    let (_output, result) = encoder.finish();
    result
}

fn db_lookup(database: &str, pattern: &str) -> std::io::Result<()> {
    let input_file = fs::File::open(db_fn(&database))?;
    let decoder = lz4::Decoder::new(input_file)?;
    let reader = io::BufReader::new(decoder);    
    let re: Regex = Regex::new(pattern).unwrap();
    
    for _line in reader.lines() {
        let line = _line.unwrap();
        if re.is_match(&line) {
            println!("{}", &line);
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let app = cli::build_cli();
    let args = app.get_matches();
    
    let database = args.value_of("database").unwrap();
    
    if args.is_present("create") {
        create_database(&database)?;
        process::exit(0);
    }
    
    if args.is_present("update") {
        update_database(&database)?;
        process::exit(0);
    }
    
    if args.is_present("list") {
        list()?;
        process::exit(0);
    }
    
    if let Some(pattern) = args.value_of("pattern") {
        //println!("Lookup: {}", pattern);
        db_lookup(&database, &pattern)?;
        process::exit(0);
    }
        
    Ok(())
}
