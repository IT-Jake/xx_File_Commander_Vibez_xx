use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use chrono::{DateTime, Local};

#[derive(Clone)]
pub struct FileItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
}

pub fn list_files(path: &PathBuf) -> Result<Vec<FileItem>, std::io::Error> {
    println!("Listing files for path: {:?}", path);
    let mut items = Vec::new();
    
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        
        // Use symlink_metadata to avoid errors on broken symlinks, 
        // or handle metadata errors gracefully.
        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => {
                // If metadata fails (e.g. broken symlink or permission denied), 
                // try symlink_metadata or skip.
                match fs::symlink_metadata(&path) {
                    Ok(m) => m,
                    Err(_) => continue, // Skip if we can't get any metadata
                }
            }
        };
        
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let is_dir = metadata.is_dir(); // Note: symlink_metadata.is_dir() is false for symlinks
        let size = metadata.len();
        
        let modified: DateTime<Local> = metadata.modified().unwrap_or(SystemTime::now()).into();
        let modified_str = modified.format("%Y-%m-%d %H:%M").to_string();

        items.push(FileItem {
            name,
            path,
            is_dir,
            size,
            modified: modified_str,
        });
    }

    // Sort: Directories first, then alphabetical
    items.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else {
            b.is_dir.cmp(&a.is_dir)
        }
    });

    Ok(items)
}

pub fn format_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if size == 0 {
        return "0 B".to_string();
    }
    let i = (size as f64).log(1024.0).floor() as usize;
    let i = std::cmp::min(i, UNITS.len() - 1);
    let p = 1024.0_f64.powi(i as i32);
    let s = size as f64 / p;
    format!("{:.2} {}", s, UNITS[i])
}

pub fn recursive_copy(source: &PathBuf, dest: &PathBuf) -> std::io::Result<()> {
    if source.is_dir() {
        if !dest.exists() {
            fs::create_dir(dest)?;
        }
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let entry_path = entry.path();
            let dest_path = dest.join(entry.file_name());
            recursive_copy(&entry_path, &dest_path)?;
        }
    } else {
        fs::copy(source, dest)?;
    }
    Ok(())
}
