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
    HashObject {
        #[clap(short, long)]
        write: bool,
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
        Command::HashObject { write, object } => {
            if write {
                let sha = create_blob_object(&object);
                println!("{}", sha);
            }
        }
    }
}

fn read_blob_object(sha: &str) -> String {
    let path = format!(".git/objects/{}/{}", &sha[0..2], &sha[2..]);
    let compressed =
        fs::read(&path).unwrap_or_else(|e| panic!("Error reading file {}: {}", path, e));
    let mut decompressed = ZlibDecoder::new(&compressed[..]);
    let mut content = Vec::new();
    decompressed.read_to_end(&mut content).unwrap();
    String::from_utf8(content)
        .unwrap()
        .splitn(2, '\0')
        .nth(1)
        .unwrap()
        .to_string()
}

fn create_blob_object(file: &str) -> String {
    //implement support for creating blob object using the gi hash-object command
    //and return the sha of the created object
    let output = std::process::Command::new("git")
        .args(&["hash-object", "-w", &file])
        .output()
        .expect("failed to execute process");

    if !output.status.success() {
        eprint!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::process::exit(1);
    }

    String::from_utf8(output.stdout).unwrap().trim().to_string()
}
