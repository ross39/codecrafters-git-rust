use clap::Parser;
use flate2::read::ZlibDecoder;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

const GIT_DIR: &str = ".git";
const GIT_OBJECTS_DIR: &str = ".git/objects";
const GIT_HEAD_FILE: &str = ".git/HEAD";
const GIT_REFS_DIR: &str = ".git/refs";
const GIT_HEAD_REF: &str = "ref: refs/heads/main\n";

#[derive(Parser)]
struct Opt {
    #[clap(subcommand)]
    command: GitCommand,
}

#[derive(Parser)]
enum GitCommand {
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
struct WriteTreeCommand {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::parse();

    match opt.command {
        GitCommand::Init(_) => init(),
        GitCommand::CatFile(cmd) => cat_file(cmd),
        GitCommand::HashObject(cmd) => hash_object(cmd),
        GitCommand::LsTree(cmd) => ls_tree(cmd),
        GitCommand::WriteTree(_) => write_tree(),
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
        let sha = create_blob_object(&cmd.object)?;
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

fn write_tree() -> Result<(), Box<dyn std::error::Error>> {
    let tree_sha = write_tree_recursive(Path::new("."))?;
    println!("{}", tree_sha);
    Ok(())
}

fn write_tree_recursive(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut tree_content = String::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_str().unwrap();

        if file_name_str == ".git" {
            continue;
        }

        let file_type = entry.file_type()?;
        let (mode, object_type, hash) = if file_type.is_dir() {
            ("040000", "tree", write_tree_recursive(&entry.path())?)
        } else {
            (
                "100644",
                "blob",
                create_blob_object(&entry.path().to_str().unwrap())?,
            )
        };

        tree_content.push_str(&format!(
            "{} {} {}\t{}\n",
            mode, object_type, hash, file_name_str
        ));
    }

    let tree_object = format!("tree {}\0{}", tree_content.len(), tree_content);

    let mut child = Command::new("git")
        .args(&["hash-object", "-w", "-t", "tree", "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(tree_object.as_bytes())?;
    let output = child.wait_with_output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create tree object: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn inspect_tree(treeish: String) -> String {
    let output = Command::new("git")
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

fn create_blob_object(file: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(&["hash-object", "-w", file])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
