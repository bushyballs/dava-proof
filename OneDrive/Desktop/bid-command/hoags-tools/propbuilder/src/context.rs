use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

/// Top-level context loaded from the JSON file passed via --context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalContext {
    pub company: CompanyInfo,
    pub signer: SignerInfo,
    #[serde(default)]
    pub past_performance: Vec<PastPerformance>,
    pub pricing: PricingInfo,
    /// Optional solicitation metadata extracted from the PDF (populated at runtime).
    #[serde(default)]
    pub solicitation: SolicitationMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyInfo {
    pub name: String,
    #[serde(default)]
    pub cage: String,
    #[serde(default)]
    pub uei: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerInfo {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PastPerformance {
    pub contract: String,
    pub title: String,
    pub value: f64,
    pub period: String,
    #[serde(default)]
    pub agency: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub poc_name: String,
    #[serde(default)]
    pub poc_phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingInfo {
    pub labor_rate: f64,
    pub overhead: f64,
    pub profit: f64,
    #[serde(default)]
    pub clins: Vec<Clin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clin {
    pub number: String,
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub unit_price: f64,
}

impl Clin {
    pub fn total(&self) -> f64 {
        self.quantity * self.unit_price
    }
}

/// Metadata extracted from (or about) the solicitation PDF.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SolicitationMeta {
    #[serde(default)]
    pub number: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub due_date: String,
    #[serde(default)]
    pub co_name: String,
    #[serde(default)]
    pub co_email: String,
    #[serde(default)]
    pub agency: String,
    #[serde(default)]
    pub issue_date: String,
}

/// Load and parse the context JSON file.
pub fn load_context(path: &Path) -> Result<ProposalContext, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(path)?;
    let ctx: ProposalContext = serde_json::from_str(&raw)?;
    Ok(ctx)
}

/// Load context and merge an optional solicitation override from a plain Value
/// (e.g. extracted from a PDF).
pub fn load_context_with_sol(
    path: &Path,
    sol: Option<SolicitationMeta>,
) -> Result<ProposalContext, Box<dyn std::error::Error>> {
    let mut ctx = load_context(path)?;
    if let Some(s) = sol {
        // Only override non-empty fields from the extracted meta.
        if !s.number.is_empty() {
            ctx.solicitation.number = s.number;
        }
        if !s.title.is_empty() {
            ctx.solicitation.title = s.title;
        }
        if !s.due_date.is_empty() {
            ctx.solicitation.due_date = s.due_date;
        }
        if !s.co_name.is_empty() {
            ctx.solicitation.co_name = s.co_name;
        }
        if !s.co_email.is_empty() {
            ctx.solicitation.co_email = s.co_email;
        }
        if !s.agency.is_empty() {
            ctx.solicitation.agency = s.agency;
        }
    }
    Ok(ctx)
}

/// Pull a dotted-path key from the raw Value representation of a context.
pub fn resolve_from_value(v: &Value, dotted_key: &str) -> Option<String> {
    hoags_core::resolve_key(v, dotted_key)
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn sample_json() -> &'static str {
        r#"{
            "company": {
                "name": "Hoags Inc.",
                "cage": "15XV5",
                "uei": "DUHWVUXFNPV5",
                "address": "123 Forest Rd, Eugene, OR 97401",
                "phone": "(458) 239-3215",
                "email": "collin@hoagsinc.com"
            },
            "signer": {
                "name": "Collin Hoag",
                "title": "President",
                "phone": "(458) 239-3215",
                "email": "collin@hoagsinc.com"
            },
            "past_performance": [
                {
                    "contract": "12444626P0025",
                    "title": "Ottawa NF Janitorial",
                    "value": 42000,
                    "period": "2026-2027",
                    "agency": "USDA Forest Service",
                    "description": "Janitorial services across 3 ranger districts."
                }
            ],
            "pricing": {
                "labor_rate": 28.00,
                "overhead": 0.10,
                "profit": 0.08,
                "clins": [
                    {
                        "number": "0001",
                        "description": "Base Year Janitorial Services",
                        "quantity": 12,
                        "unit": "Month",
                        "unit_price": 3500.00
                    }
                ]
            }
        }"#
    }

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn test_load_context_roundtrip() {
        let f = write_temp(sample_json());
        let ctx = load_context(f.path()).unwrap();
        assert_eq!(ctx.company.name, "Hoags Inc.");
        assert_eq!(ctx.company.cage, "15XV5");
        assert_eq!(ctx.signer.name, "Collin Hoag");
    }

    #[test]
    fn test_past_performance_parsed() {
        let f = write_temp(sample_json());
        let ctx = load_context(f.path()).unwrap();
        assert_eq!(ctx.past_performance.len(), 1);
        assert_eq!(ctx.past_performance[0].contract, "12444626P0025");
        assert!((ctx.past_performance[0].value - 42000.0).abs() < 0.01);
    }

    #[test]
    fn test_clin_total() {
        let clin = Clin {
            number: "0001".into(),
            description: "Base Year".into(),
            quantity: 12.0,
            unit: "Month".into(),
            unit_price: 3500.0,
        };
        assert!((clin.total() - 42000.0).abs() < 0.01);
    }

    #[test]
    fn test_pricing_defaults() {
        let f = write_temp(sample_json());
        let ctx = load_context(f.path()).unwrap();
        assert!((ctx.pricing.overhead - 0.10).abs() < 0.001);
        assert!((ctx.pricing.profit - 0.08).abs() < 0.001);
        assert_eq!(ctx.pricing.clins.len(), 1);
    }

    #[test]
    fn test_load_context_with_sol_override() {
        let f = write_temp(sample_json());
        let sol = SolicitationMeta {
            number: "SOL-2026-001".into(),
            co_name: "Ashley Stokes".into(),
            due_date: "2026-05-01".into(),
            ..Default::default()
        };
        let ctx = load_context_with_sol(f.path(), Some(sol)).unwrap();
        assert_eq!(ctx.solicitation.number, "SOL-2026-001");
        assert_eq!(ctx.solicitation.co_name, "Ashley Stokes");
    }

    #[test]
    fn test_missing_file_error() {
        let result = load_context(Path::new("/nonexistent/context.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_solicitation_defaults_to_empty() {
        let f = write_temp(sample_json());
        let ctx = load_context(f.path()).unwrap();
        assert!(ctx.solicitation.number.is_empty());
    }

    #[test]
    fn test_company_uei_present() {
        let f = write_temp(sample_json());
        let ctx = load_context(f.path()).unwrap();
        assert_eq!(ctx.company.uei, "DUHWVUXFNPV5");
    }
}
