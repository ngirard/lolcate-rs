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
use std::collections::HashMap;

mod config;
mod cli;
use crate::config::read_toml_file;
use lz4::{EncoderBuilder};


//#[macro_use] extern crate lazy_static;
extern crate regex;

use regex::Regex;

static GLOBAL_CONFIG_TEMPLATE : &str = r#"[types]
# Definition of custom file types
# Examples:
# img = ".*\\.(jp.?g|png|gif|JP.?G)$"
# video = ".*\\.(flv|mp4|mp.?g|avi|wmv|mkv|3gp|m4v|asf|webm)$"
# doc = ".*\\.(pdf|chm|epub|djvu?|mobi|azw3)$"
# audio = ".*\\.(mp3|m4a|flac|ogg)$"

"#;

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

static PROJECT_IGNORE_TEMPLATE : &str = r#"# Dirs / files to ignore.
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

fn get_db_config(toml_file: &PathBuf) -> config::Config {
    let mut buffer = String::new();
    let config: config::Config = match read_toml_file(&toml_file, &mut buffer) {
        Ok(config) => {
            config
        }
        Err(error) => {
            eprintln!("Invalid TOML: {}", error);
            process::exit(1);
        }
    };
    config
}

fn create_global_config_if_needed() -> std::io::Result<()> {
    let _fn = global_config_fn();
    if !_fn.exists() {
        fs::create_dir_all(_fn.parent().unwrap())?;
        let mut f = fs::File::create(&_fn)?;
        f.write_all(GLOBAL_CONFIG_TEMPLATE.as_bytes())?;
        println!("Created configuration file {}", _fn.display());
    }
    Ok(())
}

fn get_global_config(toml_file: &PathBuf) -> config::GlobalConfig {
    let mut buffer = String::new();
    let config: config::GlobalConfig = match read_toml_file(&toml_file, &mut buffer) {
        Ok(config) => {
            config
        }
        Err(error) => {
            eprintln!("Invalid TOML: {}", error);
            process::exit(1);
        }
    };
    config
}

fn get_types_map() -> HashMap<String, String> {
    let _fn = global_config_fn();
    let _config = get_global_config(&_fn);
    _config.types
}

fn check_db_config(config: &config::Config, toml_file: &PathBuf) {
    // Check config
    if config.dirs.len() == 0 {
        eprintln!("Please edit file {:?} and add at least a directory to scan.", toml_file);
        process::exit(1);
    }
    for dir in &config.dirs {
        if !dir.exists() {
            eprintln!("The specified dir {} doesn't exist.", dir.display());
            process::exit(1);
        }
        if !dir.is_dir() {
            eprintln!("The specified path {} is not a directory or cannot be accessed.", dir.display());
            process::exit(1);
        }
    }
}

fn global_config_fn() -> PathBuf {
    let mut _fn = lolcate_path();
    _fn.push("config.toml");
    _fn
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

fn database_names(path: PathBuf) -> Vec<(String)> {
    let mut _dbs: Vec<(String)> = Vec::new();
    let walker = walkdir::WalkDir::new(path).min_depth(1).into_iter();
    for entry in walker.filter_entry(|e| e.file_type().is_dir()) {
        if let Some(db_name) = entry.unwrap().file_name().to_str(){
            _dbs.push(db_name.to_string());
        }
    }
    _dbs
}


fn list_databases() -> std::io::Result<()> {
    let mut data: Vec<(String,String)> = Vec::new();
    let walker = walkdir::WalkDir::new(lolcate_path()).min_depth(1).into_iter();
    for entry in walker.filter_entry(|e| e.file_type().is_dir()) {
        if let Some(db_name) = entry.unwrap().file_name().to_str(){
            let config_fn = config_fn(&db_name);
            let config = get_db_config(&config_fn);
            let description = config.description;
            data.push((db_name.to_string(), description.to_string()));
        }
    }
    for (name, desc) in data {
        println!("{:width$}\t{}", name, desc, width=10);
    }
    Ok(())
}

pub fn walker(config: &config::Config, database: &str) -> ignore::Walk {
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

fn update_databases(databases: Vec<(String)>) -> std::io::Result<()> {
    for db in databases {
        update_database(&db)?;
    }
    Ok(())
}


fn update_database(database: &str) -> std::io::Result<()> {
    let config_fn = config_fn(&database);
    if !config_fn.exists() {
        eprintln!("Config file not found for database {}.\nPerhaps you forgot to type lolcate --create {} ?", &database, &database);
        process::exit(1);
    }
    let config = get_db_config(&config_fn);
    check_db_config(&config, &config_fn);
    let include_dirs = config.include_dirs;
    let ignore_symlinks = config.ignore_symlinks;
    
    let output_fn = fs::File::create(db_fn(&database))?;
    let mut encoder = EncoderBuilder::new()
        .level(4)
        .build(output_fn)?;
    
    println!("Updating {}...", database);
    for entry in walker(&config, &database) {
        let entry = match entry {
            Ok(_entry) => _entry,
            Err(err) => {
                eprintln!("failed to access entry ({})", err);
                continue;
            }
        };
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
        match entry.path().to_str() {
            Some(s) => { writeln!(encoder, "{}", s)?; },
            _ => { eprintln!("File name contains invalid unicode: {:?}", entry.path()) },
        }
    }
    
    let (_output, result) = encoder.finish();
    result
}

fn lookup_databases(databases: Vec<(String)>, pattern_re: &Regex, type_re: &Regex) -> std::io::Result<()> {
    for db in databases {
        lookup_database(&db, &pattern_re, &type_re)?;
    }
    Ok(())
}

fn lookup_database(database: &str, pattern_re: &Regex, type_re: &Regex) -> std::io::Result<()> {
    let input_file = fs::File::open(db_fn(&database))?;
    let decoder = lz4::Decoder::new(input_file)?;
    let reader = io::BufReader::new(decoder);    
    //let re: Regex = Regex::new(pattern).unwrap();
    
    for _line in reader.lines() {
        let line = _line.unwrap();
        if pattern_re.is_match(&line) && type_re.is_match(&line) {
            println!("{}", &line);
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let app = cli::build_cli();
    let args = app.get_matches();
    
    create_global_config_if_needed()?;
    
    let database = args.value_of("database").unwrap();
    let databases: Vec<(String)> = match args.is_present("all") {
        true => database_names(lolcate_path()),
        false => vec![ database.to_string() ],
    };
        
    if args.is_present("create") {
        create_database(&database)?;
        process::exit(0);
    }
    
    if args.is_present("update") {
        update_databases(databases)?;
        process::exit(0);
    }
    
    if args.is_present("list") {
        list_databases()?;
        process::exit(0);
    }

    if args.is_present("type") || args.is_present("pattern") {
        let type_re: Option<String> = match args.is_present("type") {
            false => None,
            true => {
                let type_names: Vec<&str> = args.value_of("type").unwrap().split(",").collect();
                let types_map = get_types_map();
                let type_name = type_names[0];
                match types_map.get(type_name) {
                    Some(st) => Some(st.to_string()),
                    _ => None}}};

        let type_re = match type_re {
            Some(t) => Regex::new(&t).unwrap(),
            _ => Regex::new(".").unwrap() };    
        
        let pattern_re = match args.value_of("pattern") {
            Some(t) => Regex::new(&t).unwrap(),
            _ => Regex::new(".").unwrap() };
            
        lookup_databases(databases, &pattern_re, &type_re)?;
        process::exit(0);
    }
    
    Ok(())
}
