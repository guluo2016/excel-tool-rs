use std::{env, fs, path::{Path, PathBuf}};

use anyhow::{Context, Result};
use calamine::{open_workbook_auto, Data, Reader};
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
            let df = read_excel(&file_path)?;
            // 打印前3行
            println!("{:?}", df.head(Some(3)));
            // 搜索并打印指定列
            search_df(&df).context("search df")?;
        }
    }
    Ok(())
}

fn is_excel_file<T: AsRef<Path>>(path: T) -> bool {
    let p = path.as_ref();
    if let Some(f) = p.file_name().and_then(|s| s.to_str()) {
        if f.starts_with("~$") {
            return false;
        }
    }
    if let Some(e) = p.extension().and_then(|s| s.to_str()) {
        let ext = e.to_ascii_lowercase();
        match ext.as_str() {
            "xlsx" | "xls" | "xlsb" | "xlsm" => return true,
            _ => return false,
        }
    }
    false
}

fn read_excel(path: &Path) -> Result<DataFrame> {
    let mut workbook = open_workbook_auto(path).context("open workbook")?;
    let sheet_names = workbook.sheet_names();
    let range_data = workbook.worksheet_range(&sheet_names[0])?.to_owned();
    let data: Vec<Vec<Option<String>>> = range_data.rows()
        .map(|d| {
            d.iter().map(|e| match e {
                Data::Empty => None,
                Data::String(s) => Some(s.clone()),
                Data::Float(f) => Some(f.to_string()),
                Data::Int(i) => Some(i.to_string()),
                Data::Bool(b) => Some(b.to_string()),
                Data::DateTime(dt) => Some(dt.to_string()),
                other => Some(other.to_string()),
            }).collect::<Vec<Option<String>>>()
        }).collect();
    if data.is_empty() {
        // empty sheet -> empty dataframe
        return Ok(DataFrame::new(vec![]).context("creating empty dataframe")?);
    }
    let header_row = &data[0];
    println!("Header Row: {:?}", header_row);
    let column_num = header_row.len();
    println!("Number of columns: {}", column_num);

    let mut series_vec: Vec<Series> = Vec::with_capacity(column_num);
    for (index, col_name) in header_row.iter().enumerate() {
        let col_values: Vec<Option<String>> = data.iter().skip(1).map(|d| {
                d.get(index).and_then(|v|v.clone())
            }).collect();
        // 获取col_name
        let col_name_str = match col_name {
            Some(name) => name.clone(),
            None => format!("column_{}", index + 1),
        };
        let series = Series::new(col_name_str.into(), col_values);
        series_vec.push(series);
    }
    let cols = series_vec.into_iter().map(|e| e.into())
        .collect::<Vec<_>>();
    let df = DataFrame::new(cols).context("construct datafram")?;
    Ok(df)
}

fn search_df(df: &DataFrame) -> Result<()> {
    // 搜索，且打印结果
    let aa = df.select(["集团", "大区"]).context("select columns")?;
    println!("{:?}", aa.head(Some(3)));
    Ok(())
}