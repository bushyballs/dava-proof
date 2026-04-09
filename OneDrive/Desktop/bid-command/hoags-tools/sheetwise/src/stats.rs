use crate::reader::{ColType, Sheet};
use std::collections::HashSet;

/// Extended describe()-style statistics for a numeric column.
#[derive(Debug, Clone)]
pub struct DescribeColStats {
    pub name: String,
    pub count: usize,
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub max: f64,
}

/// Compute pandas-style describe() for all numeric columns.
pub fn compute_describe(sheet: &Sheet, stats: &[ColStats]) -> Vec<DescribeColStats> {
    stats
        .iter()
        .filter(|s| s.min.is_some()) // only numeric columns
        .map(|s| {
            let col_idx = sheet
                .columns
                .iter()
                .position(|c| c.name == s.name)
                .unwrap();
            let mut nums: Vec<f64> = sheet
                .rows
                .iter()
                .filter_map(|row| {
                    let v = &row[col_idx];
                    if v.is_empty() {
                        return None;
                    }
                    let cleaned: String = v
                        .chars()
                        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                        .collect();
                    cleaned.parse::<f64>().ok()
                })
                .collect();

            nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let n = nums.len();
            let mean = if n > 0 { nums.iter().sum::<f64>() / n as f64 } else { 0.0 };
            let variance = if n > 1 {
                nums.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64
            } else {
                0.0
            };
            let std = variance.sqrt();
            let min = nums.first().copied().unwrap_or(0.0);
            let max = nums.last().copied().unwrap_or(0.0);
            let p25 = percentile(&nums, 25.0);
            let p50 = percentile(&nums, 50.0);
            let p75 = percentile(&nums, 75.0);

            DescribeColStats {
                name: s.name.clone(),
                count: n,
                mean,
                std,
                min,
                p25,
                p50,
                p75,
                max,
            }
        })
        .collect()
}

/// Linear interpolation percentile on a sorted Vec<f64>.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    let pos = (p / 100.0) * (n - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    let frac = pos - lo as f64;
    sorted[lo] + frac * (sorted[hi] - sorted[lo])
}

/// Statistics for a single column.
#[derive(Debug, Clone)]
pub struct ColStats {
    pub name: String,
    pub col_type: ColType,
    pub count: usize,
    pub missing: usize,
    pub distinct: usize,
    // Numeric only
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub sum: Option<f64>,
    pub avg: Option<f64>,
}

/// Compute per-column statistics for a sheet.
pub fn compute_stats(sheet: &Sheet) -> Vec<ColStats> {
    sheet
        .columns
        .iter()
        .map(|col| {
            let values: Vec<&str> = sheet.rows.iter().map(|r| r[col.index].as_str()).collect();
            let missing = values.iter().filter(|v| v.is_empty()).count();
            let non_empty: Vec<&str> = values.iter().copied().filter(|v| !v.is_empty()).collect();
            let count = non_empty.len();

            let distinct = non_empty
                .iter()
                .collect::<HashSet<_>>()
                .len();

            let is_numeric = matches!(col.col_type, ColType::Integer | ColType::Float);

            let (min, max, sum, avg) = if is_numeric && count > 0 {
                let nums: Vec<f64> = non_empty
                    .iter()
                    .filter_map(|v| {
                        // Strip currency/comma characters
                        let cleaned: String = v
                            .chars()
                            .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                            .collect();
                        cleaned.parse::<f64>().ok()
                    })
                    .collect();

                if nums.is_empty() {
                    (None, None, None, None)
                } else {
                    let mn = nums.iter().cloned().fold(f64::INFINITY, f64::min);
                    let mx = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    let s: f64 = nums.iter().sum();
                    let a = s / nums.len() as f64;
                    (Some(mn), Some(mx), Some(s), Some(a))
                }
            } else {
                (None, None, None, None)
            };

            ColStats {
                name: col.name.clone(),
                col_type: col.col_type.clone(),
                count,
                missing,
                distinct,
                min,
                max,
                sum,
                avg,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::read_sheet;
    use std::io::Write;

    #[test]
    fn test_stats_numeric() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "Value").unwrap();
        writeln!(tmp, "10").unwrap();
        writeln!(tmp, "20").unwrap();
        writeln!(tmp, "30").unwrap();
        let sheet = read_sheet(tmp.path()).unwrap();
        let stats = compute_stats(&sheet);
        let s = &stats[0];
        assert_eq!(s.count, 3);
        assert_eq!(s.missing, 0);
        assert!((s.min.unwrap() - 10.0).abs() < 0.001);
        assert!((s.max.unwrap() - 30.0).abs() < 0.001);
        assert!((s.sum.unwrap() - 60.0).abs() < 0.001);
        assert!((s.avg.unwrap() - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_stats_missing() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        // Write with an explicit empty field on a row (the csv crate reads a blank
        // line as zero fields when flexible=true, so use a quoted empty value instead)
        writeln!(tmp, "Name,Score").unwrap();
        writeln!(tmp, "Alice,10").unwrap();
        writeln!(tmp, "Bob,").unwrap(); // empty Score field
        writeln!(tmp, "Carol,30").unwrap();
        let sheet = read_sheet(tmp.path()).unwrap();
        let stats = compute_stats(&sheet);
        // Score column (index 1) has 1 missing
        assert_eq!(stats[1].missing, 1);
        assert_eq!(stats[1].count, 2);
    }

    #[test]
    fn test_stats_distinct() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "Cat").unwrap();
        writeln!(tmp, "A").unwrap();
        writeln!(tmp, "A").unwrap();
        writeln!(tmp, "B").unwrap();
        let sheet = read_sheet(tmp.path()).unwrap();
        let stats = compute_stats(&sheet);
        assert_eq!(stats[0].distinct, 2);
    }
}
