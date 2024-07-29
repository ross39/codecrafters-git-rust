use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest, Sha1};
use chrono::{DateTime, Local};
use std::env;
use std::fs;
use std::io::prelude::*;
use std::time;

// GitObject trait defines common methods for all Git objects
pub trait GitObject {
    fn compress(&self) -> Vec<u8>;
    fn decompress(&mut self, data: &[u8]);
    fn fmt(&self) -> &[u8];
}

// GitBlob represents a Git blob object
pub struct GitBlob {
    pub blob_data: Vec<u8>,
}

impl GitObject for GitBlob {
    fn fmt(&self) -> &[u8] {
        b"blob"
    }

    fn compress(&self) -> Vec<u8> {
        self.blob_data.clone()
    }

    fn decompress(&mut self, data: &[u8]) {
        self.blob_data = data.to_vec();
    }
}

// GitObjectType enum represents different types of Git objects
enum GitObjectType {
    Blob(GitBlob),
    Tree(GitTree),
    Commit(GitCommit),
}

// GitTreeLeaf represents a single entry in a Git tree object
#[derive(Clone)]
pub struct GitTreeLeaf {
    pub mode: Vec<u8>,
    pub path: String,
    pub sha_hash: String,
}

// Functions for parsing and handling Git tree objects
fn parse_git_treee(raw_bytes: &[u8], start_index: usize) -> (GitTreeLeaf, usize) {
    let mut index = start_index;
    let mode = parse_mode(raw_bytes, &mut index);
    let path = parse_path(raw_bytes, &mut index);
    let sha_hash = parse_sha_hash(raw_bytes, &mut index);

    (GitTreeLeaf { mode, path, sha_hash }, index)
}

fn parse_mode(raw_bytes: &[u8], index: &mut usize) -> Vec<u8> {
    let mut mode = [0; 6];
    let start_index = *index;
    while raw_bytes[*index] != b' ' {
        mode[*index - start_index] = raw_bytes[*index];
        *index += 1;
    }
    if *index - start_index == 5 {
        mode = [b'0', mode[0], mode[1], mode[2], mode[3], mode[4]];
    }
    mode.to_vec()
}

fn parse_path(raw_bytes: &[u8], index: &mut usize) -> String {
    *index += 1; // Skip whitespace
    let mut path = String::new();
    while raw_bytes[*index] != b'\0' {
        path.push(raw_bytes[*index] as char);
        *index += 1;
    }
    *index += 1; // Skip null byte
    path
}

fn parse_sha_hash(raw_bytes: &[u8], index: &mut usize) -> String {
    let mut sha_hash = String::new();
    for _ in 0..20 {
        sha_hash.push_str(&format!("{:02x}", raw_bytes[*index]));
        *index += 1;
    }
    sha_hash
}

fn git_tree_parse(raw_bytes: &[u8]) -> Vec<GitTreeLeaf> {
    let mut index = 0;
    let mut result = Vec::new();
    while index < raw_bytes.len() {
        let (leaf, new_index) = parse_git_treee(raw_bytes, index);
        result.push(leaf);
        index = new_index;
    }
    result
}

fn sort_git_tree_keys(leaf: &GitTreeLeaf) -> String {
    if leaf.mode.starts_with(b"10") {
        leaf.path.clone()
    } else {
        format!("{}\\", leaf.path)
    }
}

// GitTree represents a Git tree object
pub struct GitTree {
    pub leaves: Vec<GitTreeLeaf>,
}

impl GitObject for GitTree {
    fn fmt(&self) -> &[u8] {
        b"tree"
    }

    fn compress(&self) -> Vec<u8> {
        let sorted_leaves = {
            let mut leaves = self.leaves.clone();
            leaves.sort_by_key(sort_git_tree_keys);
            leaves
        };

        let mut result = Vec::new();
        for leaf in &sorted_leaves {
            let mode = if leaf.mode.starts_with(b"0") { &leaf.mode[1..] } else { &leaf.mode };
            result.extend_from_slice(mode);
            result.push(b' ');
            result.extend_from_slice(leaf.path.as_bytes());
            result.push(b'\0');
            let hash_bytes = hex::decode(&leaf.sha_hash).unwrap();
            result.extend_from_slice(&hash_bytes);
        }
        result
    }

    fn decompress(&mut self, data: &[u8]) {
        self.leaves = git_tree_parse(data);
    }
}

// GitCommit represents a Git commit object
pub struct GitCommit {
    pub commit_data: String,
}

impl GitObject for GitCommit {
    fn fmt(&self) -> &[u8] {
        b"commit"
    }

    fn compress(&self) -> Vec<u8> {
        self.commit_data.clone().into_bytes()
    }

    fn decompress(&mut self, data: &[u8]) {
        self.commit_data = String::from_utf8(data.to_vec()).unwrap();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args[1].as_str() {
        "init" => init_git_directory(),
        "cat-file" => cat_file(&args),
        "hash-object" => hash_object(&args),
        "ls-tree" => ls_tree(&args),
        "write-tree" => write_new_git_tree_command(),
        "commit-tree" => commit_tree(&args),
        _ => println!("Unknown command: {}", args[1]),
    }
}

// Initialize a new Git repository
fn init_git_directory() {
    fs::create_dir(".git").unwrap();
    fs::create_dir(".git/objects").unwrap();
    fs::create_dir(".git/refs").unwrap();
    fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
    println!("Initialized git directory");
}

// Display the contents of a Git object
fn cat_file(args: &[String]) {
    let hash = &args[args.len() - 1];
    let object = read_object_from_store(hash);
    match object {
        GitObjectType::Blob(blob) => {
            std::io::stdout().write_all(&blob.compress()).unwrap();
            std::io::stdout().flush().unwrap();
        }
        _ => println!("Unexpected object type for cat-file"),
    }
}

// Hash the contents of a file and store it as a Git object
fn hash_object(args: &[String]) {
    let file_path = &args[args.len() - 1];
    let data = fs::read(file_path).unwrap();
    let object = GitBlob { blob_data: data };
    let contents = object.compress();
    let object_type = object.fmt();
    let hash = write_object_to_store(contents.as_slice(), object_type);
    println!("{}", hash);
}

// List the contents of a Git tree object
fn ls_tree(args: &[String]) {
    let hash = &args[args.len() - 1];
    let object = read_object_from_store(hash);
    match object {
        GitObjectType::Tree(tree) => ls_tree_contents(tree),
        _ => println!("Not a tree object"),
    }
}

fn ls_tree_contents(tree: GitTree) {
    for leaf in tree.leaves {
        println!("{}", leaf.path);
    }
}

// Write the current directory structure as a Git tree object
fn write_new_git_tree_command() {
    let tree_hash = write_new_git_tree(".");
    println!("{}", tree_hash);
}

// Create a new commit object
fn commit_tree(args: &[String]) {
    let parent_hash_index = args.iter().position(|x| x == "-p").unwrap();
    let parent_hash = &args[parent_hash_index + 1];
    let message_index = args.iter().position(|x| x == "-m").unwrap();
    let message = &args[message_index + 1];
    let commit_tree_index = args.iter().position(|x| x == "commit-tree").unwrap();
    let tree_hash = &args[commit_tree_index + 1];
    let commit_hash = commit(tree_hash, message, parent_hash);
    println!("{}", commit_hash);
}

// Read a Git object from the object store
fn read_object_from_store(hash: &str) -> GitObjectType {
    let path = format!(".git/objects/{}/{}", &hash[..2], &hash[2..]);
    let data = fs::read(path).unwrap();
    let mut decoder = ZlibDecoder::new(data.as_slice());
    let mut decoded_bytes = Vec::new();
    decoder.read_to_end(&mut decoded_bytes).unwrap();

    let index_of_first_whitespace = decoded_bytes.iter().position(|&x| x == b' ').unwrap();
    let index_of_first_null = decoded_bytes.iter().position(|&x| x == 0).unwrap();
    let object_type = &decoded_bytes[..index_of_first_whitespace];
    let byte_contents = &decoded_bytes[index_of_first_null + 1..];

    match object_type {
        b"blob" => {
            let mut blob = GitBlob { blob_data: Vec::new() };
            blob.decompress(byte_contents);
            GitObjectType::Blob(blob)
        }
        b"tree" => {
            let mut tree = GitTree { leaves: Vec::new() };
            tree.decompress(byte_contents);
            GitObjectType::Tree(tree)
        }
        _ => panic!("Unknown object type"),
    }
}

// Write a Git object to the object store
fn write_object_to_store(contents: &[u8], object_type: &[u8]) -> String {
    let mut result = Vec::new();
    result.extend_from_slice(object_type);
    result.push(b' ');
    result.extend_from_slice(contents.len().to_string().as_bytes());
    result.push(b'\0');
    result.extend_from_slice(contents);

    let mut hasher = Sha1::new();
    hasher.update(&result);
    let hash_result = hasher.finalize();
    let sha_string = hex::encode(hash_result);

    let path = format!(".git/objects/{}/{}", &sha_string[..2], &sha_string[2..]);
    fs::create_dir_all(format!(".git/objects/{}", &sha_string[..2])).unwrap();

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&result).unwrap();
    let compressed = encoder.finish().unwrap();
    fs::write(path, compressed).unwrap();

    sha_string
}

// Write a directory structure as a Git tree object
fn write_new_git_tree(path: &str) -> String {
    let mut entries: Vec<(Vec<u8>, String, String)> = Vec::new();

    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();
        let mode = if metadata.is_dir() { b"040000".to_vec() } else { b"100644".to_vec() };
        let file_name = entry.file_name().into_string().unwrap();
        let entry_path = entry.path();

        if file_name == ".git" {
            continue;
        }

        if metadata.is_dir() {
            let tree_sha_hash = write_new_git_tree(entry_path.to_str().unwrap());
            entries.push((mode, file_name, tree_sha_hash));
        } else {
            let blob_contents = fs::read(entry_path.clone()).unwrap();
            let blob = GitBlob { blob_data: blob_contents };
            let blob_contents = blob.compress();
            let sha_hash = write_object_to_store(blob_contents.as_slice(), blob.fmt());
            entries.push((mode, file_name, sha_hash));
        }
    }

    let tree = GitTree {
        leaves: entries
            .iter()
            .map(|(mode, entry_path, sha_hash)| GitTreeLeaf {
                mode: mode.clone(),
                path: entry_path.clone(),
                sha_hash: sha_hash.clone(),
            })
            .collect(),
    };

    let tree_ser = tree.compress();
    write_object_to_store(tree_ser.as_slice(), tree.fmt())
}

// Create a new commit object
write_new_git_commit(tree_hash: &str, message: &str, parent_hash: &str) -> String {
    let hardcoded_author_name = "Kevin Guo";
    let hardcoded_author_email = "kev.guo123@gmail.com";

    let timestamp = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let now: DateTime<Local> = Local::now();
    let offset = now.offset();
    let offset_hours = offset.local_minus_utc() / 3600;
    let offset_minutes = (offset.local_minus_utc() % 3600) / 60;
    let offset = format!("{:+03}{:02}", offset_hours, offset_minutes);

    let author_contents = format!(
        "{} <{}> {} {}",
        hardcoded_author_name, hardcoded_author_email, timestamp, offset
    );

    let commit_lines = vec![
        format!("tree {}", tree_hash),
        format!("parent {}", parent_hash),
        format!("author {}", author_contents),
        format!("committer {}", author_contents),
        "".to_string(),
        message.to_string(),
        "".to_string(),
    ];

    let commit_contents = commit_lines.join("\n");
    let commit = GitCommit { commit_data: commit_contents };
    let commit_contents = commit.compress();
    write_object_to_store(commit_contents.as_slice(), commit.fmt())
}