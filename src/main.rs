use std::path::Path;

use clap::{App, Arg};
use composition_analyzer::CompositionAnalyzer;

use crate::area::Area;

mod area;
mod chunk;
mod chunk_loader;
mod chunk_section;
mod composition_analyzer;
mod layers;
mod palette;

fn main() {
    let matches = App::new("mca-analyzer")
        .version("0.1.0")
        .about("Analyze Minecraft's .mca region files")
        .arg(
            Arg::with_name("folder")
                .help("The region folder to be analyzed")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("An optional output file")
                .takes_value(true),
        )
        .get_matches();

    let input_path = if let Some(folder) = matches.value_of("folder") {
        let path = Path::new(folder);
        if !path.is_dir() {
            eprintln!("'{}' is not a folder!", folder);
            return;
        }
        path
    } else {
        eprintln!("No input folder has been specified.");
        return;
    };

    let _output_path = if let Some(output_file) = matches.value_of("output") {
        Some(Path::new(output_file))
    } else {
        None
    };

    let mut composition_analyzer =
        CompositionAnalyzer::new(input_path.as_os_str().to_str().unwrap());

    let area = Area::new(0, 256, 0, 256);
    composition_analyzer.analyze(area);

    composition_analyzer.print_csv();
}
