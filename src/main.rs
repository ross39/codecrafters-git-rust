use clap::Parser;
use flate2::read::ZlibDecoder;
use std::fs;
use std::io::Read;

#[derive(Parser)]
struct Opt {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    Init,
    CatFile {
        #[clap(short, long)]
        pretty: bool,
        object: String,
    },
}

fn main() {
    let opt = Opt::parse();

    match opt.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory");
        }
        Command::CatFile { pretty, object } => {
            if pretty {
                let content = read_blob_object(&object);
                print!("{}", content);
            }
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