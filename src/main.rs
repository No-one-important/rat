use bincode::{self, deserialize, serialize};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Files to archive only used for archiving not extracting
    #[arg(short, long, num_args(0..))]
    input_files: Option<Vec<String>>,

    /// Output file name for archiving and input file for extracting
    #[arg(short, long)]
    archive_file: String,

    /// Extract from archive
    #[arg(short = 'x', long)]
    extract: bool,

    /// Compress archive
    #[arg(short, long)]
    compress: bool,
}

#[derive(Serialize, Deserialize)]
struct FileInfo {
    file_name: String,
    data_len: u64,
}

struct File {
    info: FileInfo,
    data: Vec<u8>,
}

fn main() {
    let args = Args::parse();
    if args.extract {
        extract(&args.archive_file);
    } else {
        archive(
            args.input_files.expect("Input files required"),
            &args.archive_file,
            args.compress
        )
    }
}

fn extract(archive_file: &str) {
    let mut data = fs::read(archive_file).expect("Archive file not found");
    let compressed = data.pop().unwrap();
    if compressed == 1 {
        data = decompress_size_prepended(&data).unwrap();
    }

    loop {
        //get len of file_info
        let len = u64::from_be_bytes(data[0..8].try_into().unwrap()) as usize;
        data.drain(0..8);

        //get file info
        let file_info = &data[0..len];
        let file_info: FileInfo = deserialize(file_info).unwrap();
        data.drain(0..len);

        // get data and write to file
        fs::write(file_info.file_name, &data[0..(file_info.data_len as usize)]).unwrap();
        data.drain(0..(file_info.data_len as usize));

        // break when all files finished
        if data.len() == 0 {
            break;
        }
    }
}

fn archive(file_names: Vec<String>, output_file: &str, compress: bool) {
    let mut files = vec![];

    for file_name in file_names {
        let data = fs::read(&file_name).unwrap();

        let info = FileInfo {
            file_name: file_name,
            data_len: data.len() as u64,
        };

        files.push(File {
            info: info,
            data: data,
        });
    }

    // output file buffer
    let mut output: Vec<u8> = vec![];

    for mut file in files {
        // encode info and calculate length
        let mut file_info = serialize(&file.info).unwrap();
        let len = file_info.len().to_be_bytes();

        // write length + info to buffer
        output.append(&mut Vec::from(len));
        output.append(&mut file_info);

        // write data to buffer
        output.append(&mut file.data);
    }

    // compress and add marker if compressed or not
    if compress {
        output = compress_prepend_size(&output);
        output.push(1);
    } else {
        output.push(0);
    }


    fs::write(output_file, output).unwrap();
}
