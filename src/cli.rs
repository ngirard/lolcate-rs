extern crate clap;
use clap::{Arg, App}; // SubCommand

pub fn build_cli() -> App<'static, 'static> {
    App::new("Lolcate")
        .version("0.1")
        .author("Nicolas Girard <girard.nicolas@gmail.com>")
        .about("Find files by name -- A better locate / mlocate / updatedb")
        .arg(Arg::with_name("update")
            .help("Update database")
            .long("update")
            //.value_name("DB")
            .takes_value(false)
            //.default_value("default")
            .conflicts_with_all(&["pattern",])
            .required(false)
            //.index(1)
            )
            
        .arg(Arg::with_name("database")
            .help("Database to use")
            .long("db")
            .takes_value(true)
            .required(false)
            .default_value("default"))
            
        .arg(Arg::with_name("pattern")
            .value_name("PATTERN")
            .index(1))
}
