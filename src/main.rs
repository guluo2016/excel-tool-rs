use std::{env, fs, path::{Path, PathBuf}};

use anyhow::{Context, Result};
use calamine::{open_workbook_auto, Data, Range, Reader};
use polars::prelude::*;

fn main() -> Result<()> {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_path = project_root.join("data");
    println!("Data directory path: {}", data_path.display());

    if !data_path.exists() || !data_path.is_dir() {
        panic!("Data directory does not exist or is not a directory");
    }

    for entry in fs::read_dir(&data_path).context("reading data directory")? {
        let entry = entry.context("reading directory entry")?;
        let file_path = entry.path();
        if is_excel_file(&file_path) {
            let sheets = read_excel_to_dataframes(&file_path)
                .with_context(|| format!("reading excel: {}", file_path.display()))?;
            for (sheet_name, df) in sheets {
                println!("File: {} | Sheet: {} | shape: {:?}", file_path.display(), sheet_name, df.shape());
                println!("Preview:\n{:?}", df.head(Some(5)));
            }
        }
    }
    Ok(())
}

fn is_excel_file<P: AsRef<Path>>(path: P) -> bool {
    let p = path.as_ref();
    if let Some(fname) = p.file_name().and_then(|s| s.to_str()) {
        if fname.starts_with("~$") {
            return false;
        }
    }
    p.extension()
        .and_then(|e| e.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "xlsx" | "xls" | "xlsb" | "xlsm"))
        .unwrap_or(false)
}

fn read_excel_to_dataframes(path: &Path) -> Result<Vec<(String, DataFrame)>> {
    let mut workbook = open_workbook_auto(path).context("open workbook")?;
    let mut out = Vec::new();
    for sheet_name in workbook.sheet_names().to_owned() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            let df = range_to_dataframe(&sheet_name, &range).with_context(|| format!("convert sheet {}", sheet_name))?;
            out.push((sheet_name.clone(), df));
        }
    }
    Ok(out)
}

fn range_to_dataframe(_name: &str, range: &Range<Data>) -> Result<DataFrame> {
    // Collect rows as Vec<Vec<Option<String>>>; convert cells to strings and treat empty as None
    let rows: Vec<Vec<Option<String>>> = range
        .rows()
        .map(|r| {
            r.iter()
                .map(|c| match c {
                    Data::Empty => None,
                    Data::String(s) => Some(s.clone()),
                    Data::Float(f) => Some(f.to_string()),
                    Data::Int(i) => Some(i.to_string()),
                    Data::Bool(b) => Some(b.to_string()),
                    Data::DateTime(dt) => Some(dt.to_string()),
                    other => Some(other.to_string()),
                })
                .collect()
        })
        .collect();

    if rows.is_empty() {
        // empty sheet -> empty dataframe
        return Ok(DataFrame::new(vec![]).context("creating empty dataframe")?);
    }

    // first row as header (if present); fallback to column_{i}
    let header_row = &rows[0];
    let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0);

    let mut series_vec: Vec<Series> = Vec::with_capacity(ncols);
    for col_idx in 0..ncols {
        let col_name = header_row
            .get(col_idx)
            .and_then(|opt| opt.as_ref())
            .map(|s| s.clone())
            .unwrap_or_else(|| format!("column_{}", col_idx + 1));

        // collect column values (skip header row)
        let col_vals: Vec<Option<String>> = rows
            .iter()
            .skip(1)
            .map(|r| r.get(col_idx).and_then(|v| v.clone()))
            .collect();

        // Series::new accepts Vec<Option<String>>; use .into() for the name
        let s = Series::new(col_name.into(), col_vals);
        series_vec.push(s);
    }

    // DataFrame::new expects a Vec<Column> so convert Series into Columns
    let cols = series_vec.into_iter().map(|s| s.into()).collect::<Vec<_>>();
    let df = DataFrame::new(cols).context("construct dataframe")?;
    Ok(df)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_excel_file_accepts() {
        assert!(is_excel_file(PathBuf::from("foo.xlsx")));
        assert!(is_excel_file(PathBuf::from("foo.XLSX")));
        assert!(is_excel_file(PathBuf::from("bar.xls")));
    }

    #[test]
    fn test_is_excel_file_rejects_temp_and_dirs() {
        assert!(!is_excel_file(PathBuf::from("~$temp.xlsx")));
        assert!(!is_excel_file(PathBuf::from("some_folder")));
    }
}