/*
 * This file is part of Lolcate.
 *
 * Copyright © 2019 Nicolas Girard
 *
 * ActivityPub is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * ActivityPub is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with ActivityPub.  If not, see <http://www.gnu.org/licenses/>.
 */

extern crate dirs;
extern crate expanduser;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate lz4;
extern crate ignore;
extern crate walkdir;
extern crate termcolor;

use std::path::{PathBuf, MAIN_SEPARATOR};
use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::io;
use std::io::prelude::*;
use std::process;
use std::str;
use std::collections::HashMap;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod config;
mod cli;
use crate::config::read_toml_file;
use lz4::{EncoderBuilder};


#[macro_use] extern crate lazy_static;
extern crate regex;
use regex::{Regex, RegexBuilder};

static GLOBAL_CONFIG_TEMPLATE : &str = r#"[types]
# Definition of custom path name types
# Examples:
# img = ".*\\.(jp.?g|png|gif|JP.?G)$"
# video = ".*\\.(flv|mp4|mp.?g|avi|wmv|mkv|3gp|m4v|asf|webm)$"
# doc = ".*\\.(pdf|chm|epub|djvu?|mobi|azw3|odf|ods|md|tex|txt)$"
# audio = ".*\\.(mp3|m4a|flac|ogg)$"

"#;

static PROJECT_CONFIG_TEMPLATE : &str = r#"
description = ""

# Directories to index.
dirs = [
  # "~/first/dir",
  # "/second/dir"
]

# Set to either "Dirs" or "Files" to skip directories or files
# skip = "Dirs"

# Set to true if you want skip symbolic links
ignore_symlinks = false

# Set to true if you want to ignore hidden files and directories
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
    path.push("lolcate");
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
    
    println!("Created database '{}'.\nPlease edit:", db_name);
    println!("- the configuration file: {}", config_fn.display());
    println!("- the ignores file:       {}", ignores_fn.display());
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


fn info_databases() -> std::io::Result<()> {
    let mut db_data: Vec<(String, String, String, String)> = Vec::new();
    let walker = walkdir::WalkDir::new(lolcate_path()).min_depth(1).into_iter();
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let mut section_spec = ColorSpec::new();
    section_spec.set_fg(Some(Color::Cyan));
    let mut entry_spec = ColorSpec::new();
    entry_spec.set_fg(Some(Color::Green));
    stdout.set_color(&section_spec)?;
    writeln!(&mut stdout, "Config file:")?;
    stdout.reset()?;
    writeln!(&mut stdout, "  {}\n", global_config_fn().display())?;
    for entry in walker.filter_entry(|e| e.file_type().is_dir()) {
        if let Some(db_name) = entry.unwrap().file_name().to_str(){
            let config_fn = config_fn(&db_name);
            let config = get_db_config(&config_fn);
            let description = config.description;
            db_data.push((db_name.to_string(), description.to_string(),
                          config_fn.display().to_string(), ignores_fn(&db_name).display().to_string()));
        }
    }
    stdout.set_color(&section_spec)?;
    match db_data.len() {
        0 => {
            writeln!(&mut stdout, "No databases found.")?;
        },
        _ => {
            writeln!(&mut stdout, "Databases:")?;
            stdout.reset()?;
            for (name, desc, config, ignores) in db_data {
                stdout.set_color(&entry_spec)?;
                writeln!(&mut stdout, "  {}", name)?;
                stdout.reset()?;
                println!("    Description:  {}", desc);
                println!("    Config file:  {}", config);
                println!("    Ignores file: {}", ignores);
            }
        }
    };
    let tm = get_types_map();
    stdout.set_color(&section_spec)?;
    println!("");
    match tm.len() {
        0 => {
            writeln!(&mut stdout, "No file types found.")?;
        },
        _ => {
            writeln!(&mut stdout, "File types:")?;
            stdout.reset()?;
            for (name, glob) in tm {
                stdout.set_color(&entry_spec)?;
                write!(&mut stdout, "  {}", name)?;
                stdout.reset()?;
                println!(": {}", glob);
            }
        }};
    stdout.reset()?;
    println!("");
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
        eprintln!("Config file not found for database {}.\nPerhaps you forgot to run lolcate --create {} ?", &database, &database);
        process::exit(1);
    }
    let config = get_db_config(&config_fn);
    check_db_config(&config, &config_fn);
    let skip = config.skip;
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
        if skip!=config::Skip::None || ignore_symlinks {
            if let Some(ft) = entry.file_type() {
                if ft.is_dir() {
                    if skip==config::Skip::Dirs { continue };
                } else {
                    if skip==config::Skip::Files { continue };
                }
                if ignore_symlinks && ft.is_symlink() {
                    continue;
                }
            } else {
                continue;
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

fn build_regex(pattern: &str, ignore_case: bool) -> Regex {
    lazy_static! {
        static ref UPPER_RE: Regex = Regex::new(r"[[:upper:]]").unwrap();
    };
    let re: Regex = match RegexBuilder::new(pattern)
        .case_insensitive(ignore_case || !UPPER_RE.is_match(&pattern))
        .build() {
            Ok(re) => {
                re
            }
            Err(error) => {
                eprintln!("Invalid regex: {}", error);
                process::exit(1);
            }};
    re
}

fn basename<'a>(line: &'a str) -> Cow<'a, str> {
    let mut pieces = line.rsplit(MAIN_SEPARATOR);
    match pieces.next() {
        Some(p) => p.into(),
        None => line.into(),
    }
}

fn lookup_databases(databases: Vec<(String)>, patterns_re: &Vec<(Regex)>, types_re: &Vec<Regex>, bn_patterns_re: &Vec<Regex>) -> std::io::Result<()> {
    for db in databases {
        lookup_database(&db, patterns_re, &types_re, &bn_patterns_re)?;
    }
    Ok(())
}

fn lookup_database(database: &str, patterns_re: &Vec<(Regex)>, types_re: &Vec<Regex>, bn_patterns_re: &Vec<Regex>) -> std::io::Result<()> {
    let db_file = db_fn(&database);
    if !db_file.parent().unwrap().exists() {
        eprintln!("Database {} doesn't exist. Perhaps you forgot to run lolcate --create {} ?", &database, &database);
        process::exit(1);
    }
    if !db_file.exists() {
        eprintln!("Database {} is empty. Perhaps you forgot to run lolcate --update {} ?", &database, &database);
        process::exit(1);
    }
    let input_file = fs::File::open(db_file)?;
    let decoder = lz4::Decoder::new(input_file)?;
    let reader = io::BufReader::new(decoder);
    let stdout = io::stdout();
    let lock = stdout.lock();
    let mut w = io::BufWriter::new(lock);
    for _line in reader.lines() {
        let line = _line.unwrap();
        let mut matched = false;
        if types_re.len()>0 {
            for type_re in types_re {
                if type_re.is_match(&line) { matched = true; break };
            }
            if !matched { continue };
        }
        matched = false;
        for re in patterns_re {
            match re.is_match(&line) {
                false => { matched = false; break },
                true => { matched = true; }
            };
        }
        if !matched { continue };
        if bn_patterns_re.len()>0 {
            matched = false;
            for bn_pattern_re in bn_patterns_re {
                match bn_pattern_re.is_match(&basename(&line)) {
                    false => { matched = false; break },
                    true => { matched = true; }
                };
            }
            if !matched { continue };
        }
        if matched {
            writeln!(&mut w, "{}", &line).unwrap();
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
    
    if args.is_present("info") {
        info_databases()?;
        process::exit(0);
    }

    // lookup
    let types_re: Vec<(regex::Regex)> = match args.value_of("type") {
        None => vec![],
        Some(vals) => {
            let types_map = get_types_map();
            vals.split(",").map(|n| types_map.get(n))
                .filter(|t| t.is_some())
                .map(|t| t.unwrap())
                .map(|t| Regex::new(&t).unwrap())
                .collect()
        }};
    let patterns = args.values_of("pattern").map(|vals| vals.collect::<Vec<_>>());
    let patterns_re = match patterns {
        None => vec![ Regex::new(".").unwrap() ],
        Some(patterns) => patterns.into_iter().map(|p| build_regex(&p, args.is_present("ignore_case"))).collect() };
    
    let bn_patterns = args.values_of("basename_pattern").map(|vals| vals.collect::<Vec<_>>());
    let bn_patterns_re = match bn_patterns {
        None => vec![],
        Some(patterns) => patterns.into_iter().map(|p| build_regex(&p, args.is_present("ignore_case"))).collect() };
    
    lookup_databases(databases, &patterns_re, &types_re, &bn_patterns_re)?;
    Ok(())
}
