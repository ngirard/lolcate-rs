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

use std::collections::HashMap;
use std::process;
use std::{convert, fs, io::prelude::*, path};
use toml::de::Error;
use toml::macros::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub description: String,
    #[serde(deserialize_with = "deserialize::deserialize")]
    pub dirs: Vec<path::PathBuf>,
    #[serde(default = Skip::None)]
    pub skip: Skip,
    pub ignore_symlinks: bool,
    pub ignore_hidden: bool,
}

#[derive(Debug, Deserialize, PartialEq, Copy, Clone)]
pub enum Skip {
    None,
    Dirs,
    Files,
}

#[derive(Debug, Deserialize)]
pub struct GlobalConfig {
    pub types: HashMap<String, String>,
}

pub fn read_toml_file<'a, 'de, P: ?Sized, T>(
    path: &'a P,
    buffer: &'de mut String,
) -> Result<T, Error>
where
    P: convert::AsRef<path::Path>,
    T: Deserialize<'de>,
{
    let mut configuration_file: fs::File = match fs::OpenOptions::new().read(true).open(path) {
        Ok(val) => val,
        Err(_e) => {
            eprintln!("Cannot open file {}", path.as_ref().display());
            process::exit(1);
        }
    };

    match configuration_file.read_to_string(buffer) {
        Ok(_bytes) => toml::from_str(buffer.as_str()),
        Err(error) => panic!(
            "The data in this stream is not valid UTF-8.\nSee error: '{}'\n",
            error
        ),
    }
}

mod deserialize {
    use serde::de::{Deserialize, Deserializer};
    use std::path;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<path::PathBuf>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Vec::<String>::deserialize(deserializer)?;
        s.into_iter()
            .map(|s| expanduser::expanduser(&s).map_err(serde::de::Error::custom))
            .collect()
    }
}
