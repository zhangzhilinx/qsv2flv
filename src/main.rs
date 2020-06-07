#[macro_use]
extern crate clap;

use std::fs::File;
use std::path::Path;
use std::process;

use crate::convert_qsv::{
    meta_data_from_tag_blocks, tag_blocks_from_qsv, validate_qsv_format, write_from_qsv_to_flv,
};

mod convert_qsv;
mod error;
mod flv_format;
#[macro_use]
mod macros;

// 爱奇艺的QSV视频格式实际上是F4V格式，对原视频信息进行了混淆和干扰
// 本程序的反混淆处理针对的是第2版本的qsv文件（可能无法处理2016年之前的视频）
// 是相关github项目的Rust版本

pub fn convert_qsv_to_flv(qsv: &mut File, flv: &mut File, verbose: bool) -> error::Result<()> {
    cond!(verbose, println!("[STEP] Validate qsv file..."));
    validate_qsv_format(qsv)?;
    cond!(verbose, println!("[STEP] Parse all tag blocks..."));
    let tags = tag_blocks_from_qsv(qsv)?;
    cond!(
        verbose,
        println!("[STEP] Generate metadata from all tag blocks...")
    );
    let meta = meta_data_from_tag_blocks(qsv, &tags)?;
    cond!(verbose, println!("[STEP] Write from qsv to flv..."));
    write_from_qsv_to_flv(qsv, &tags, flv, &meta)?;
    flv.sync_all()?;

    Ok(())
}

fn main() {
    let mut verbose = false;
    let matches = clap_app!(qsv2flv =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
        (about: "A tool for converting QSV to FLV")
        // (@arg CONFIG: -c --config +takes_value "Sets a custom config file")
        (@arg INPUT: +required "Sets the input file to use")
        (@arg OUTPUT: +required "Sets the output file to use")
        // (@arg debug: -d --debug ... "Sets the level of debugging information")
        (@arg verbose: -v --verbose "Print test information verbosely")
    )
        .get_matches();

    cond!(matches.is_present("verbose"), verbose = true);

    let qsv_path = Path::new(matches.value_of("INPUT").unwrap());
    let flv_path = Path::new(matches.value_of("OUTPUT").unwrap());

    let mut qsv = match File::open(&qsv_path) {
        Err(why) => {
            eprintln!("[ERROR] Couldn't open {}: {}", qsv_path.display(), why);
            process::exit(1);
        }
        Ok(file) => file,
    };
    let mut flv = match File::create(&flv_path) {
        Err(why) => {
            eprintln!("[ERROR] Couldn't open {}: {}", flv_path.display(), why);
            process::exit(1);
        }
        Ok(file) => file,
    };

    let time_s = std::time::SystemTime::now();
    match convert_qsv_to_flv(&mut qsv, &mut flv, verbose) {
        Err(why) => {
            eprintln!(
                "[ERROR] Couldn't convert {} to {}: {}",
                qsv_path.display(),
                flv_path.display(),
                why
            );
            process::exit(1);
        }
        Ok(file) => file,
    }
    let time_e = std::time::SystemTime::now();
    if verbose {
        println!(
            "[INFO] Time cost: {:?}s",
            time_e.duration_since(time_s).unwrap().as_millis() as f64 / 1000f64
        );
    }
}
