// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::env;

use fs_extra::dir;

fn main() {
    let mut target_dir_path = env::var("OUT_DIR").unwrap();
    target_dir_path.push_str("/../../../");

    let mut options = dir::CopyOptions::new();
    options.overwrite = true;
    dir::copy("include/", target_dir_path, &options).unwrap();
}
