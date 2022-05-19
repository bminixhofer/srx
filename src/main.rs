use std::fs::{File, read_to_string};
use std::io::prelude::*;
use std::io::LineWriter;
use std::io::{self, BufRead};
use std::path::Path;
use std::str::FromStr;
use srx::SRX;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(short, long, default_value="en")]
    language: String,
    #[clap(short, long)]
    input: String,
    #[clap(short, long)]
    output: String,
    #[clap(short, long)]
    srxfile: String,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Prepare output file to be written segment by segment
    let file = File::create(args.output)?;
    let mut file = LineWriter::new(file);
    // Load SRX rules from file
    let rules =
        SRX::from_str(&read_to_string(args.srxfile).expect("rules file exists"))
            .expect("srx rule file is valid")
            .language_rules(args.language);

    // Read each input file line (it could be a whole document)
    if let Ok(lines) = read_lines(args.input) {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(doc) = line {
                // Split the content using the SRX segmenter and write each segment to the output
                for splittedline in rules.split(&doc).collect::<Vec<_>>(){
                    file.write(splittedline.as_bytes())?;
                    file.write(b"\n")?;
                }
            }
        }
    }

    file.flush()?;
    Ok(())

}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}