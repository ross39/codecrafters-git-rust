use std::fs;
use std::path::{Path, PathBuf};
use sha1::{Sha1, Digest};

fn write_tree(_: WriteTreeCommand) -> Result<(), Box<dyn std::error::Error>> {
    let tree_hash = write_tree_recursive(Path::new("."))?;
    println!("{}", tree_hash);
    Ok(())
}

fn write_tree_recursive(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();

        // Skip .git directory and hidden files
        if file_name.starts_with('.') {
            continue;
        }

        if path.is_file() {
            let blob_hash = create_blob_object(&path)?;
            entries.push((String::from("100644"), String::from("blob"), blob_hash, file_name.to_string()));
        } else if path.is_dir() {
            let tree_hash = write_tree_recursive(&path)?;
            entries.push((String::from("40000"), String::from("tree"), tree_hash, file_name.to_string()));
        }
    }

    // Sort entries as Git does
    entries.sort_by(|a, b| a.3.cmp(&b.3));

    create_tree_object(&entries)
}

fn create_tree_object(entries: &[(String, String, String, String)]) -> Result<String, Box<dyn std::error::Error>> {
    let mut tree_content = Vec::new();
    for (mode, object_type, hash, name) in entries {
        let entry = format!("{} {} {}\t{}", mode, object_type, hash, name);
        tree_content.extend_from_slice(entry.as_bytes());
        tree_content.push(0); // Null byte separator
    }

    let header = format!("tree {}", tree_content.len());
    let mut content = header.into_bytes();
    content.push(0);
    content.extend_from_slice(&tree_content);

    let mut hasher = Sha1::new();
    hasher.update(&content);
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);

    // Write the tree object
    let object_path = format!(".git/objects/{}/{}", &hash_hex[..2], &hash_hex[2..]);
    fs::create_dir_all(Path::new(&object_path).parent().unwrap())?;
    fs::write(&object_path, zlib::compress(&content))?;

    Ok(hash_hex)
}

fn create_blob_object(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let content = fs::read(path)?;
    let header = format!("blob {}", content.len());
    let mut blob = header.into_bytes();
    blob.push(0);
    blob.extend_from_slice(&content);

    let mut hasher = Sha1::new();
    hasher.update(&blob);
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);

    // Write the blob object
    let object_path = format!(".git/objects/{}/{}", &hash_hex[..2], &hash_hex[2..]);
    fs::create_dir_all(Path::new(&object_path).parent().unwrap())?;
    fs::write(&object_path, zlib::compress(&blob))?;

    Ok(hash_hex)
}