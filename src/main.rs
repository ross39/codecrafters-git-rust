use flate2::read::ZlibDecoder;
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::io::Read;
fn main() {
    //  You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    let args: Vec<String> = env::args().collect();
    if args[1] == "init" {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
        println!("Initialized git directory")
    } else if args[1] == "cat-file" {
        // cat-file -p <blob_sha>
        if args[2] == "-p" {
            let sha = &args[3];
            let content = read_blob_object(sha);
            println!("{}", content);
        }
}
}

    

// This funcion is used to read a git blob object
fn read_blob_object(sha: &str) -> String {
    let path = format!(".git/objects/{}", sha);
    let compressed = fs::read(path).unwrap();
    let mut decompressed = ZlibDecoder::new(&compressed[..]);
    let mut content = Vec::new();
    decompressed.read_to_end(&mut content).unwrap();
    String::from_utf8(content).unwrap()
}
