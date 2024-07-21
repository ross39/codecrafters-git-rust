#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
mod git;
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    //println!("Logs from your program will appear here!");
    // Uncomment this block to pass the first stage
    let args: Vec<String> = env::args().collect();
    if args[1] == "init" {
        git::init(".");
    } else if args[1] == "cat-file" {
        if args[2] == "-p" {
            print!("{}", git::cat_file(".", &args[3]));
        }
    } else if args[1] == "hash-object" {
        if args[2] == "-w" {
            use std::fs::File;
            use std::io::Read;
            let mut file = File::open(&args[3]).expect("Failed to open file");
            let mut contents = std::vec::Vec::new();
            file.read_to_end(&mut contents)
                .expect("Failed to read from file");
            print!("{}", git::hash_object(".", "blob", &contents));
        }
    } else if args[1] == "ls-tree" {
        if args[2] == "--name-only" {
            let dirs = git::ls_tree(".", &args[3]);
            for d in dirs {
                println!("{}", d);
            }
        }
    } else if args[1] == "write-tree" {
        let sha = git::write_tree(".");
        println!("{}", sha);
    } else {
        println!("unknown command2: {}", args[1])
    }
}
