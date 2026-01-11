use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    println!("Project root directory: {}", project_root.display());
    let data_path = project_root.join("data");
    println!("Data directory path: {}", data_path.display());

    if !data_path.exists() || !data_path.is_dir() {
        panic!("Data directory does not exist or is not a directory");
    }

    let entries = fs::read_dir(&data_path)?;
    for entry in entries {
        let entry = entry?;
        let file_path = entry.path();
        println!("Found file: {}", file_path.display());
        if is_excel_file(&file_path) {
            println!("Excel file detected: {}", file_path.display());
        }
    }
    Ok(())
}

fn is_excel_file(file_path: &PathBuf) -> bool {
    if let Some(extension) = file_path.extension() {
        if extension == "xlsx" || extension == "xls" {
            if let Some(f) = file_path.file_name() {
                if let Some(fname) = f.to_str() {
                    if !fname.starts_with("~$") {
                        return true;
                    }
                }
            }
        }
    }
    false
}