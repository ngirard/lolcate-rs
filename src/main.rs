/*
 * This file is part of Lolcate.
 *
 * Copyright © 2019 Nicolas Girard
 *
 * Lolcate is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Lolcate is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Lolcate.  If not, see <http://www.gnu.org/licenses/>.
 */

use crate::config::read_toml_file;
extern crate crossbeam_channel as channel;
use lazy_static::lazy_static;
use lz4::EncoderBuilder;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::str;
use std::thread;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod cli;
mod config;

use regex::{Regex, RegexBuilder};

static GLOBAL_CONFIG_TEMPLATE: &str = r#"[types]
# Definition of custom path name types
# Examples:
img = ".*\\.(jp.?g|png|gif|JP.?G)$"
video = ".*\\.(flv|mp4|mp.?g|avi|wmv|mkv|3gp|m4v|asf|webm)$"
doc = ".*\\.(pdf|chm|epub|djvu?|mobi|azw3|odf|ods|md|tex|txt|adoc)$"
audio = ".*\\.(mp3|m4a|flac|ogg)$"

"#;

static PROJECT_CONFIG_TEMPLATE: &str = r#"
description = ""

# Directories to index.
dirs = [
  # "~/first/dir",
  # "/second/dir"
]

# Set to "Dirs" or "Files" to skip directories or files.
# If unset, or set to "None", both files and directories will be included.
# skip = "Dirs"

# Set to true if you want skip symbolic links
ignore_symlinks = false

# Set to true if you want to ignore hidden files and directories
ignore_hidden = false

# Set to true to read .gitignore files and ignore matching files
gitignore = false

"#;

static PROJECT_IGNORE_TEMPLATE: &str = r#"# Dirs / files to ignore.
# Use the same syntax as gitignore(5).
# Common patterns:
#
# .git
# *~
"#;

pub fn lolcate_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap();
    path.push("lolcate");
    path
}

pub fn lolcate_data_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap();
    path.push("lolcate");
    path
}

fn get_db_config(toml_file: &PathBuf) -> config::Config {
    let mut buffer = String::new();
    let config: config::Config = match read_toml_file(&toml_file, &mut buffer) {
        Ok(config) => config,
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
        Ok(config) => config,
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
        eprintln!(
            "Please edit file {:?} and add at least a directory to scan.",
            toml_file
        );
        process::exit(1);
    }
    for dir in &config.dirs {
        if !dir.exists() {
            eprintln!("The specified dir {} doesn't exist.", dir.display());
            process::exit(1);
        }
        if !dir.is_dir() {
            eprintln!(
                "The specified path {} is not a directory or cannot be accessed.",
                dir.display()
            );
            process::exit(1);
        }
    }
}

fn global_config_fn() -> PathBuf {
    let mut _fn = lolcate_config_path();
    _fn.push("config.toml");
    _fn
}

fn config_fn(db_name: &str) -> PathBuf {
    let mut _fn = lolcate_config_path();
    _fn.push(db_name);
    _fn.push("config.toml");
    _fn
}

fn db_fn(db_name: &str) -> PathBuf {
    let mut _fn = lolcate_data_path();
    _fn.push(db_name);
    _fn.push("db.lz4");
    _fn
}

fn ignores_fn(db_name: &str) -> PathBuf {
    let mut _fn = lolcate_config_path();
    _fn.push(db_name);
    _fn.push("ignores");
    _fn
}

fn create_database(db_name: &str) -> std::io::Result<()> {
    let mut db_dir = lolcate_data_path();
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

fn database_names(path: PathBuf) -> Vec<String> {
    let mut _dbs: Vec<String> = Vec::new();
    let walker = walkdir::WalkDir::new(path).min_depth(1).into_iter();
    for entry in walker.filter_entry(|e| e.file_type().is_dir()) {
        if let Some(db_name) = entry.unwrap().file_name().to_str() {
            _dbs.push(db_name.to_string());
        }
    }
    _dbs
}

fn info_databases() -> std::io::Result<()> {
    let mut db_data: Vec<(String, String, String, String, String)> = Vec::new();
    let walker = walkdir::WalkDir::new(lolcate_config_path())
        .min_depth(1)
        .into_iter();
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
        if let Some(db_name) = entry.unwrap().file_name().to_str() {
            let config_fn = config_fn(&db_name);
            let config = get_db_config(&config_fn);
            let description = config.description;
            let mut db_fn = lolcate_data_path();
            db_fn.push(db_name);
            db_data.push((
                db_name.to_string(),
                description.to_string(),
                config_fn.display().to_string(),
                ignores_fn(&db_name).display().to_string(),
                db_fn.display().to_string(),
            ));
        }
    }
    stdout.set_color(&section_spec)?;
    match db_data.len() {
        0 => {
            writeln!(&mut stdout, "No databases found.")?;
        }
        _ => {
            writeln!(&mut stdout, "Databases:")?;
            stdout.reset()?;
            for (name, desc, config, ignores, db_fn) in db_data {
                stdout.set_color(&entry_spec)?;
                writeln!(&mut stdout, "  {}", name)?;
                stdout.reset()?;
                println!("    Description:  {}", desc);
                println!("    Config file:  {}", config);
                println!("    Ignores file: {}", ignores);
                println!("    Data file:    {}", db_fn);
            }
        }
    };
    let tm = get_types_map();
    stdout.set_color(&section_spec)?;
    println!();
    match tm.len() {
        0 => {
            writeln!(&mut stdout, "No file types found.")?;
        }
        _ => {
            writeln!(&mut stdout, "File types:")?;
            stdout.reset()?;
            for (name, glob) in tm {
                stdout.set_color(&entry_spec)?;
                write!(&mut stdout, "  {}", name)?;
                stdout.reset()?;
                println!(": {}", glob);
            }
        }
    };
    stdout.reset()?;
    println!();
    Ok(())
}

pub fn walker(config: &config::Config, database: &str) -> ignore::WalkParallel {
    let paths = &config.dirs;
    let mut wd = ignore::WalkBuilder::new(&paths[0]);
    wd.hidden(config.ignore_hidden) // Whether to ignore hidden files
        .parents(false) // Don't read ignore files from parent directories
        .follow_links(!config.ignore_symlinks) // Follow symbolic links
        .ignore(true) // Don't read .ignore files
        .git_global(config.gitignore) // Don't read global gitignore file
        .git_ignore(config.gitignore) // Don't read .gitignore files
        .git_exclude(false); // Don't read .git/info/exclude files

    for path in &paths[1..] {
        wd.add(path);
    }
    wd.add_ignore(ignores_fn(&database));
    wd.threads(4);
    wd.build_parallel()
}

fn update_databases(databases: Vec<String>) -> std::io::Result<()> {
    for db in databases {
        update_database(&db)?;
    }
    Ok(())
}

fn update_database(db_name: &str) -> std::io::Result<()> {
    let config_fn = config_fn(&db_name);
    if !config_fn.exists() {
        eprintln!("Config file not found for database {}.\nPerhaps you forgot to run lolcate --create {} ?", &db_name, &db_name);
        process::exit(1);
    }
    let config = get_db_config(&config_fn);
    check_db_config(&config, &config_fn);
    let skip = config.skip;
    let ignore_symlinks = config.ignore_symlinks;
    let db_path = db_fn(&db_name);
    let parent_path = db_path.parent().unwrap();
    if !parent_path.exists() {
        fs::create_dir_all(parent_path)?;
    }
    let output_fn = fs::File::create(db_path)?;
    let (tx, rx) = channel::bounded::<ignore::DirEntry>(8000);

    println!("Updating {}...", db_name);

    let stdout_thread = thread::spawn(move || {
        let mut encoder = EncoderBuilder::new()
            .level(3)
            .block_mode(lz4::BlockMode::Linked)
            .block_size(lz4::BlockSize::Max256KB)
            .build(output_fn)?;
        for entry in rx {
            match entry.path().to_str() {
                Some(s) => {
                    writeln!(encoder, "{}", s).unwrap();
                }
                _ => eprintln!("File name contains invalid unicode: {:?}", entry.path()),
            }
        }
        let (_output, result) = encoder.finish();
        result
    });

    walker(&config, &db_name).run(|| {
        let tx = tx.clone();
        Box::new(move |entry| { //: Result<ignore::DirEntry,ignore::Error>
            use ignore::WalkState::*;
            let entry = match entry {
                    Ok(_entry) => _entry,
                Err(err) => {
                    eprintln!("failed to access entry ({})", err);
                    return Continue
                }
            };
            if skip != config::Skip::None || ignore_symlinks {
                if let Some(ft) = entry.file_type() {
                    if ft.is_dir() {
                        if skip == config::Skip::Dirs {
                            return Continue;
                        };
                    } else {
                        if skip == config::Skip::Files {
                            return Continue;
                        };
                    }
                    if ignore_symlinks && ft.is_symlink() {
                        return Continue;
                    }
                } else {
                    return Continue;
                }
            }
            match tx.send(entry) {
                Ok(_) => ignore::WalkState::Continue,
                Err(_) => ignore::WalkState::Quit,
            }
        })
    });
    drop(tx);
    stdout_thread.join().unwrap()
}

fn build_regex(pattern: &str, ignore_case: bool) -> Regex {
    lazy_static! {
        static ref UPPER_RE: Regex = Regex::new(r"[[:upper:]]").unwrap();
    };
    let re: Regex = match RegexBuilder::new(pattern)
        .case_insensitive(ignore_case || !UPPER_RE.is_match(&pattern))
        .build()
    {
        Ok(re) => re,
        Err(error) => {
            eprintln!("Invalid regex: {}", error);
            process::exit(1);
        }
    };
    re
}

fn lookup_databases(
    db_names: Vec<String>,
    patterns_re: &[Regex],
    types_re: &[Regex],
) -> std::io::Result<()> {
    for db_name in db_names {
        lookup_database(&db_name, patterns_re, &types_re)?;
    }
    Ok(())
}

fn lookup_database(
    db_name: &str,
    patterns_re: &[Regex],
    types_re: &[Regex],
) -> std::io::Result<()> {
    let db_file = db_fn(&db_name);
    if !db_file.parent().unwrap().exists() {
        eprintln!(
            "Database {} doesn't exist. Perhaps you forgot to run lolcate --create {} ?",
            &db_name, &db_name
        );
        process::exit(1);
    }
    if !db_file.exists() {
        eprintln!(
            "Database {} is empty. Perhaps you forgot to run lolcate --update {} ?",
            &db_name, &db_name
        );
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
        if types_re.len() > 0 {
            if !types_re.iter().any(|re| re.is_match(&line)) {
                continue;
            }
        }
        if !patterns_re.iter().all(|re| re.is_match(&line)) {
            continue;
        }

        writeln!(&mut w, "{}", &line).unwrap();
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let app = cli::build_cli();
    let args = app.get_matches();

    create_global_config_if_needed()?;

    let database = args.value_of("database").unwrap();
    let databases: Vec<String> = match args.is_present("all") {
        true => database_names(lolcate_config_path()),
        false => vec![database.to_string()],
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
    let types_map = get_types_map();
    let types_re = args
        .value_of("type")
        .unwrap_or_default()
        .split(",")
        .map(|n| types_map.get(n))
        .filter(|t| t.is_some())
        .map(|t| t.unwrap())
        .map(|t| Regex::new(&t).unwrap())
        .collect::<Vec<_>>();

    let ignore_case = args.is_present("ignore_case");

    let patterns_re = args
        .values_of("pattern")
        .unwrap_or_default()
        .map(|p| build_regex(p, ignore_case));

    let bn_patterns_re = args
        .values_of("basename_pattern")
        .unwrap_or_default()
        .map(|p| build_regex(&format!("/[^/]*{}[^/]*$", p), ignore_case));

    lookup_databases(
        databases,
        &patterns_re.chain(bn_patterns_re).collect::<Vec<_>>(),
        &types_re,
    )?;
    Ok(())
}
