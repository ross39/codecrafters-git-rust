use clap::Parser;
use flate2::read::ZlibDecoder;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

//TODO: Implement the following git commands
const GIT_DIR: &str = ".git";
const GIT_OBJECTS_DIR: &str = ".git/objects";
const GIT_HEAD_FILE: &str = ".git/HEAD";
const GIT_REFS_DIR: &str = ".git/refs";
const GIT_HEAD_REF: &str = "ref: refs/heads/main\n";
#[derive(Parser)]
struct Opt {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    Init(InitCommand),
    CatFile(CatFileCommand),
    HashObject(HashObjectCommand),
    LsTree(LsTreeCommand),
    WriteTree(WriteTreeCommand),
}

#[derive(Parser)]
struct InitCommand {}

#[derive(Parser)]
struct CatFileCommand {
    #[clap(short, long)]
    pretty: bool,
    object: String,
}

#[derive(Parser)]
struct HashObjectCommand {
    #[clap(short, long)]
    write: bool,
    object: String,
}

#[derive(Parser)]
struct LsTreeCommand {
    #[clap(long)]
    name_only: bool,
    #[clap(name = "treeish")]
    treeish: String,
}

#[derive(Parser)]
struct WriteTreeCommand {

}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::parse();

    match opt.command {
        Command::Init(_) => init(),
        Command::CatFile(cmd) => cat_file(cmd),
        Command::HashObject(cmd) => hash_object(cmd),
        Command::LsTree(cmd) => ls_tree(cmd),
        Command::WriteTree(cmd) => write_tree(cmd),
    }
}

fn init() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir(GIT_DIR)?;
    fs::create_dir(GIT_OBJECTS_DIR)?;
    fs::create_dir(GIT_REFS_DIR)?;
    fs::write(GIT_HEAD_FILE, GIT_HEAD_REF)?;
    Ok(())
}

fn cat_file(cmd: CatFileCommand) -> Result<(), Box<dyn std::error::Error>> {
    if cmd.pretty {
        let object = read_blob_object(&cmd.object);
        print!("{}", object);
    } 
    Ok(())
}

fn hash_object(cmd: HashObjectCommand) -> Result<(), Box<dyn std::error::Error>> {
    if cmd.write {
        let sha = create_blob_object(&cmd.object);
        println!("{}", sha);
    }
    Ok(())
}

fn ls_tree(cmd: LsTreeCommand) -> Result<(), Box<dyn std::error::Error>> {
    let output = inspect_tree(cmd.treeish);
    if cmd.name_only {
        for line in output.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 2 {
                println!("{}", parts[1]);
            }
        }
    } else {
        println!("{}", output);
    }
    Ok(())
}
/*
Iterate over the files/directories in the working directory

If the entry is a file, create a blob object and record its SHA hash

If the entry is a directory, recursively create a tree object and record its SHA hash

Once you have all the entries and their SHA hashes, write the tree object to the .git/objects directory
*/
fn write_tree(_: WriteTreeCommand) -> Result<(), Box<dyn std::error::Error>> {
    let entries = fs::read_dir(".")?;
    let mut tree_entries = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let path_str = path.as_path().display().to_string();

        if path.is_file() {
            let blob_hash = create_blob_object(&path_str);
            tree_entries.push((path, blob_hash));
        } else if path.is_dir() {
            let tree_hash = write_tree_for_directory(&path_str)?;
            tree_entries.push((path, tree_hash));
        }
    }

    let tree_hash = create_tree_object(&tree_entries)?;
    println!("{}", tree_hash);
    Ok(())
}

fn write_tree_for_directory(dir: &str) -> Result<String, Box<dyn std::error::Error>> {
    //recursively create a tree object for each directory and record its SHA hash
    let entries = fs::read_dir(dir)?;
    let mut tree_entries = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let path_str = path.as_path().display().to_string();

        if path.is_file() {
            let blob_hash = create_blob_object(&path_str);
            tree_entries.push((path, blob_hash));
        } else if path.is_dir() {
            let tree_hash = write_tree_for_directory(&path_str)?;
            tree_entries.push((path, tree_hash));
        }
    }
    create_tree_object(&tree_entries).expect("Error creating tree object for directory");
    Ok("".to_string())
}

fn create_tree_object(entries: &Vec<(&str, PathBuf, String)>) -> Result<String, Box<dyn std::error::Error>> {
    let mut tree_content = Vec::new();
    for (entry_type, path, hash) in entries {
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let entry = format!("100644 {} {}\t{}", entry_type, hash, file_name);
        tree_content.push(entry);
    }

    let tree_content = tree_content.join("\n");

    let mut child = std::process::Command::new("git")
        .args(&["hash-object", "-w", "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    child.stdin.as_mut().unwrap().write_all(tree_content.as_bytes()).expect("failed to write to stdin");

    let output = child.wait_with_output().expect("failed to wait on child");

    if !output.status.success() {
        eprint!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::process::exit(1);
    }

    let tree_hash = String::from_utf8(output.stdout).unwrap().trim().to_string();
    Ok(tree_hash)
}
fn inspect_tree(treeish: String) -> String {
    let output = std::process::Command::new("git")
        .args(&["ls-tree", &treeish])
        .output()
        .expect("failed to execute process");

    if !output.status.success() {
        eprint!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::process::exit(1);
    }

    String::from_utf8(output.stdout).unwrap()
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
