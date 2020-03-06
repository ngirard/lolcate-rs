# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [0.6.0pre] – 2020-03-06

### Added

- The new `gitignore` option, when set to `true` in `config.toml`, enables Lolcate to take into account `.gitignore` files and skip the paths that match one of the `.gitignore` patterns.

### Changed

- Use regex for basename matching instead of splitting the path, leading to a 10% performance improvement (ec5140f, thanks to @icewind1991).
- Added `adoc` (Asciidoc) to the `doc` predefined path type.


### Fixed

- Fix error when no `skip` directive in config.toml (#7).
- Create db path if necessary (#8).

## [0.5.0] – 2019-09-03

### Changed

- Performance improvements.
- A number of path types come predefined:
  ```
    img = ".*\\.(jp.?g|png|gif|JP.?G)$"
    video = ".*\\.(flv|mp4|mp.?g|avi|wmv|mkv|3gp|m4v|asf|webm)$"
    doc = ".*\\.(pdf|chm|epub|djvu?|mobi|azw3|odf|ods|md|tex|txt)$"
    audio = ".*\\.(mp3|m4a|flac|ogg)$"
  ```

  (to be used with e.g. `lolcate --type doc <pattern>`).
- Configuration and data files are now split in separate directories (#5).

  This requires migrating your existing files. On Linux, you can use the following script:

  ```sh
    migrate_lolcate_data(){
        lolcate_data_dir=${XDG_DATA_HOME:-$HOME/.local/share}/lolcate
        lolcate_conf_dir=${XDG_CONFIG_HOME:-$HOME/.config}/lolcate
        mkdir ${lolcate_conf_dir}
        mv ${lolcate_data_dir}/config.toml ${lolcate_conf_dir}
        ls -d $lolcate_data_dir/*/ | while read db_dir; do
            db_name=$(basename $db_dir)
            db_config_dir=${lolcate_conf_dir}/${db_name}
            mkdir ${db_config_dir}
            mv ${db_dir}/{config.toml,ignores} ${db_config_dir}
        done
    }
    migrate_lolcate_data
  ```
