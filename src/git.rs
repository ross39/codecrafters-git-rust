fn compress(data: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data)
        .expect("Failed to write object type to encoder");
    encoder.finish().expect("Failed to finish compression")
}
fn decompress(data: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibDecoder;
    use std::io::Write;
    let mut decoder = ZlibDecoder::new(Vec::new());
    decoder
        .write_all(data)
        .expect("Failed to write object type to decoder");
    decoder.finish().expect("Failed to finish decompression")
}
fn read_file<P: AsRef<std::path::Path>>(path: P) -> Vec<u8> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).expect("Failed to open file");
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .expect("Failed to read from file");
    contents
}
fn sha_hash(data: &[u8]) -> [u8; 20] {
    use sha1::{Digest, Sha1};
    use std::convert::TryInto;
    let mut hasher = Sha1::new();
    hasher.update(&data);
    hasher.finalize().as_slice().try_into().unwrap()
}
fn sha_hash_str(data: &[u8]) -> String {
    hex::encode(sha_hash(data))
}
fn object_dir<P: AsRef<std::path::Path>>(repo_dir: P, sha: &str) -> std::path::PathBuf {
    repo_dir
        .as_ref()
        .join(".git")
        .join("objects")
        .join(&sha[..2])
}
fn object_file<P: AsRef<std::path::Path>>(repo_dir: P, sha: &str) -> std::path::PathBuf {
    object_dir(repo_dir, sha).join(&sha[2..])
}
struct Object {
    bytes: Vec<u8>,
    //object_type: String,
    space_idx: usize,
    contents_idx: usize,
    contents_size: usize,
}
fn read_object<P: AsRef<std::path::Path>>(repo_dir: P, sha: &str) -> Object {
    let object_file_path = object_file(repo_dir, &sha);
    let compressed_bytes = read_file(object_file_path);
    let decompressed_bytes = decompress(&compressed_bytes);
    let space_idx = decompressed_bytes
        .iter()
        .position(|x| *x == b' ')
        .expect("Could not find space");
    let null_idx = space_idx
        + decompressed_bytes
            .iter()
            .skip(space_idx)
            .position(|x| *x == b'\0')
            .expect("Could not find null terminator");
    let size_str = std::str::from_utf8(&decompressed_bytes[space_idx + 1..null_idx])
        .expect("Unable to parse size");
    let contents_size: usize = size_str.parse().expect("Unable to parse size string");
    //	let object_type = std::str::from_utf8(&decompressed_bytes[..space_idx]).expect("Unable to parse object type").to_string();
    Object {
        bytes: decompressed_bytes,
        //object_type,
        space_idx,
        contents_idx: null_idx + 1,
        contents_size,
    }
}
pub fn init<P: AsRef<std::path::Path>>(path: P) {
    use std::fs;
    println!("{:?}", path.as_ref().join(".git"));
    fs::create_dir(path.as_ref().join(".git")).unwrap();
    fs::create_dir(path.as_ref().join(".git").join("objects")).unwrap();
    fs::create_dir(path.as_ref().join(".git").join("refs")).unwrap();
    fs::write(
        path.as_ref().join(".git").join("HEAD"),
        "ref: refs/heads/master\n",
    )
    .unwrap();
    println!("Initialized git directory in {:?}", path.as_ref())
}
pub fn cat_file<P: AsRef<std::path::Path>>(path: P, sha: &str) -> String {
    let object: Object = read_object(path, &sha);
    let contents_slice = &object.bytes[object.contents_idx..];
    if object.contents_size != contents_slice.len() {
        panic!("Size does not match contents size")
    }
    let contents = std::str::from_utf8(contents_slice).expect("Failed to convert contents");
    match object.bytes[..object.space_idx] {
        [b'b', b'l', b'o', b'b'] => contents.to_string(),
        _ => panic!("Could not match object type"),
    }
}
pub fn hash_object<P: AsRef<std::path::Path>>(
    path: P,
    object_type: &str,
    contents: &[u8],
) -> String {
    use std::fs::File;
    use std::io::Write;
    let mut object = Vec::new();
    object.extend_from_slice(object_type.as_bytes());
    object.extend_from_slice(&[b' ']);
    object.extend_from_slice(contents.len().to_string().as_bytes());
    object.extend_from_slice(&[b'\0']);
    object.extend_from_slice(&contents);
    let compressed_contents = compress(&object);
    let sha = sha_hash_str(&object);
    let sha_dir = object_dir(&path, &sha);
    std::fs::create_dir(&sha_dir).expect("Failed to create sha object directory");
    let mut file =
        File::create(object_file(path, &sha)).expect("Failed to create test object file");
    file.write_all(&compressed_contents)
        .expect("Failed to write test object file");
    sha
}
fn parse_one_tree(bytes: &[u8], index: usize) -> (&[u8], usize, String) {
    let space_idx = index
        + bytes
            .iter()
            .skip(index)
            .position(|x| *x == b' ')
            .expect("Could not find space");
    let null_idx = space_idx
        + bytes
            .iter()
            .skip(space_idx)
            .position(|x| *x == b'\0')
            .expect("Could not find null terminator");
    (
        &bytes[index..space_idx],
        null_idx + 21,
        std::str::from_utf8(&bytes[space_idx + 1..null_idx])
            .expect("Unable to parse directory string")
            .to_string(),
    )
}
fn parse_tree(bytes: &[u8]) -> Vec<String> {
    let mut result = Vec::new();
    let max = bytes.len();
    let mut idx = 0;
    while idx < max {
        let r = parse_one_tree(&bytes, idx);
        idx = r.1;
        result.push(r.2);
    }
    result
}
pub fn ls_tree<P: AsRef<std::path::Path>>(path: P, sha: &str) -> Vec<String> {
    let object: Object = read_object(path, &sha);
    parse_tree(&object.bytes[object.contents_idx..])
}
pub fn write_tree<P: AsRef<std::path::Path>>(path: P) -> String {
    use std::os::unix::fs::PermissionsExt;
    let paths = std::fs::read_dir(path.as_ref()).expect("Failed to read directory");
    fn add_path(contents: &mut Vec<u8>, mode: &str, path: &str) {
        contents.extend_from_slice(mode.as_bytes());
        contents.extend_from_slice(&[b' ']);
        contents.extend_from_slice(path.as_bytes());
        contents.extend_from_slice(&[b'\0']);
        contents.extend_from_slice(&sha_hash(path.as_bytes())); // random hash
    }
    let mut contents = Vec::new();
    let mut files: Vec<(String, String)> = Vec::new();
    for p in paths {
        let file_name = p
            .as_ref()
            .unwrap()
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        if !file_name.starts_with(".") {
            let metadata = std::fs::metadata(&file_name).unwrap();
            let permissions = metadata.permissions();
            let mode = format!("{:0>6o}", permissions.mode());
            files.push((file_name, mode));
        }
    }
    files.sort_by(|x, y| x.0.cmp(&y.0));
    for f in files {
        add_path(&mut contents, &f.1, &f.0);
    }
    return hash_object(path.as_ref(), "tree", &contents);
}
#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempdir::TempDir;
    #[test]
    fn test_init() {
        let temp_dir = TempDir::new("test").unwrap();
        init(&temp_dir);
        assert!(temp_dir.path().join(".git").exists());
        assert!(temp_dir.path().join(".git/objects").exists());
        assert!(temp_dir.path().join(".git/refs").exists());
        let head = std::fs::read_to_string(temp_dir.path().join(".git/HEAD"))
            .expect("Failed to read .git/HEAD");
        assert!(head == "ref: refs/heads/master\n");
    }
    #[test]
    fn test_cat_file() {
        let temp_dir = TempDir::new("test").unwrap();
        init(&temp_dir);
        let contents: &str = temp_dir
            .path()
            .to_str()
            .expect("Failed to convert temp dir to string");
        let sha = {
            let mut full_contents = Vec::new();
            full_contents.extend_from_slice("blob ".as_bytes());
            full_contents.extend_from_slice(contents.len().to_string().as_bytes());
            full_contents.extend_from_slice(&[b'\0']);
            full_contents.extend_from_slice(contents.as_bytes());
            let sha = sha_hash_str(&full_contents);
            let sha_dir = temp_dir.path().join(".git").join("objects").join(&sha[..2]);
            std::fs::create_dir(&sha_dir).expect("Failed to create sha object directory");
            let mut file =
                File::create(sha_dir.join(&sha[2..])).expect("Failed to create test object file");
            let compressed_contents = compress(&full_contents);
            file.write_all(&compressed_contents)
                .expect("Failed to write test object file");
            sha
        };
        let cat_contents = cat_file(temp_dir.path(), &sha);
        assert_eq!(contents, cat_contents);
    }
    #[test]
    fn test_hash_object() {
        let temp_dir = TempDir::new("test").unwrap();
        init(&temp_dir);
        let contents = temp_dir
            .path()
            .to_str()
            .expect("Failed to convert temp dir to string")
            .as_bytes();
        let object_sha = hash_object(&temp_dir, "blob", contents);
        let mut full_contents = Vec::new();
        full_contents.extend_from_slice("blob ".as_bytes());
        full_contents.extend_from_slice(contents.len().to_string().as_bytes());
        full_contents.extend_from_slice(&[b'\0']);
        full_contents.extend_from_slice(contents);
        let sha = sha_hash_str(&full_contents);
        assert_eq!(sha, object_sha);
        let object_file = temp_dir
            .path()
            .join(".git")
            .join("objects")
            .join(&sha[..2])
            .join(&sha[2..]);
        let compressed_contents = compress(&full_contents);
        let file_contents = read_file(object_file);
        assert_eq!(file_contents, compressed_contents);
    }
    #[test]
    fn test_ls_tree() {
        let temp_dir = TempDir::new("test").unwrap();
        init(&temp_dir);
        fn add_path(contents: &mut Vec<u8>, mode: &str, path: &str) {
            contents.extend_from_slice(mode.as_bytes());
            contents.extend_from_slice(&[b' ']);
            contents.extend_from_slice(path.as_bytes());
            contents.extend_from_slice(&[b'\0']);
            contents.extend_from_slice(&sha_hash(path.as_bytes())); // random hash
        }
        let mut contents = Vec::new();
        //[mode] space [path] 0x00 [sha-1]
        add_path(&mut contents, "000000", "test_dir");
        add_path(&mut contents, "100000", "test_file");
        add_path(&mut contents, "000000", "dir");
        let sha = hash_object(&temp_dir, "tree", &contents);
        let directories = ls_tree(&temp_dir, &sha);
        assert_eq!(directories.len(), 2);
        assert_eq!(directories[0], "test_dir");
        assert_eq!(directories[1], "dir");
    }
}
