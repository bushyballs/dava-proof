use crate::reader::Sheet;
use crate::stats::ColStats;
use serde_json::{json, Value};
use tabled::{
    builder::Builder,
    settings::{Alignment, Style},
};

/// Print rows as an aligned table to stdout.
pub fn print_table(columns: &[String], rows: &[Vec<String>]) {
    let mut builder = Builder::default();
    builder.push_record(columns);
    for row in rows {
        let record: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
        builder.push_record(record);
    }
    let mut table = builder.build();
    table
        .with(Style::modern())
        .with(Alignment::left());
    println!("{table}");
}

/// Print rows as a JSON array of objects to stdout.
pub fn print_json(columns: &[String], rows: &[Vec<String>]) {
    let array: Vec<Value> = rows
        .iter()
        .map(|row| {
            let mut obj = serde_json::Map::new();
            for (i, col) in columns.iter().enumerate() {
                let val = row.get(i).map(|s| s.as_str()).unwrap_or("");
                // Try to preserve numeric types in JSON
                if let Ok(n) = val.parse::<i64>() {
                    obj.insert(col.clone(), json!(n));
                } else if let Ok(f) = val.parse::<f64>() {
                    obj.insert(col.clone(), json!(f));
                } else {
                    obj.insert(col.clone(), json!(val));
                }
            }
            Value::Object(obj)
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&array).unwrap());
}

/// Print rows as CSV to stdout.
pub fn print_csv(columns: &[String], rows: &[Vec<String>]) {
    let mut wtr = csv::WriterBuilder::new().from_writer(std::io::stdout());
    wtr.write_record(columns).unwrap();
    for row in rows {
        wtr.write_record(row).unwrap();
    }
    wtr.flush().unwrap();
}

/// Print the `info` summary for a sheet.
pub fn print_info(sheet: &Sheet) {
    println!("File  : {}", sheet.path);
    println!(
        "Delim : {}",
        match sheet.delimiter {
            b',' => "comma (,)",
            b'\t' => "tab (\\t)",
            b'|' => "pipe (|)",
            b';' => "semicolon (;)",
            d => return println!("Delim : {:?}", d as char),
        }
    );
    println!("Rows  : {}", sheet.row_count());
    println!("Cols  : {}", sheet.col_count());
    println!();
    println!("{:<30} {}", "Column", "Type");
    println!("{}", "-".repeat(42));
    for col in &sheet.columns {
        println!("{:<30} {}", col.name, col.col_type);
    }
}

/// Print stats table.
pub fn print_stats(stats: &[ColStats]) {
    println!(
        "{:<25} {:<10} {:>8} {:>8} {:>14} {:>14} {:>14} {:>8} {:>8}",
        "Column", "Type", "Count", "Missing", "Min", "Max", "Sum", "Avg", "Distinct"
    );
    println!("{}", "-".repeat(110));
    for s in stats {
        let fmt_num = |v: Option<f64>| match v {
            Some(n) if n.fract() == 0.0 && n.abs() < 1e12 => format!("{:.0}", n),
            Some(n) => format!("{:.2}", n),
            None => "-".to_string(),
        };
        println!(
            "{:<25} {:<10} {:>8} {:>8} {:>14} {:>14} {:>14} {:>8} {:>8}",
            truncate(&s.name, 24),
            format!("{}", s.col_type),
            s.count,
            s.missing,
            fmt_num(s.min),
            fmt_num(s.max),
            fmt_num(s.sum),
            fmt_num(s.avg),
            s.distinct,
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
