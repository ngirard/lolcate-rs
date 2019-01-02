extern crate dirs;
use std::path::PathBuf;
use std::fs;
use std::fs::File;
use std::io::Write;

static DEFAULT_PROJECT_TEMPLATE : &str = r#"
[dirs]
# Dirs to add

[ignore]
# Files / dirs to ignore

"#;


pub fn lolcate_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap();
    path.push("lolcate-rs");
    println!("{:?}", path);
    path
}

fn main() -> std::io::Result<()> {
    let mut default_config_file = lolcate_path();
    default_config_file.push("config.toml");
    if !default_config_file.exists() {
        fs::create_dir_all(default_config_file.parent().unwrap());
        let mut f = File::create(&default_config_file)?;
        f.write_all(DEFAULT_PROJECT_TEMPLATE.as_bytes())?;
        println!("Added default database.\nPlease edit {:?}.", default_config_file);
    }
    Ok(())
}
