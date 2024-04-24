use flate2::read::ZlibDecoder;
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::io::Read;
fn main() {
    // println!("Logs from your program will appear here!");

    let args: Vec<String> = env::args().collect();
    if args[1] == "init" {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
        println!("Initialized git directory")
    } else if args[1] == "cat-file" {
        if args[2] == "-p" {
            let sha = &args[3];
            let content = read_blob_object(sha);
            println!("{}", content);
        }
    }
}

fn read_blob_object(sha: &str) -> String {
    let dir = &sha[0..2];
    let filename = &sha[2..];
    let path = format!(".git/objects/{}/{}", dir, filename);
    let compressed = match fs::read(&path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file {}: {}", path, e);
            std::process::exit(1);
        }
    };
    let mut decompressed = ZlibDecoder::new(&compressed[..]);
    let mut content = Vec::new();
    decompressed.read_to_end(&mut content).unwrap();
    let content_str = String::from_utf8(content).unwrap();
    let null_index = content_str.find('\0').unwrap();
    content_str[(null_index + 1)..].to_string()
}