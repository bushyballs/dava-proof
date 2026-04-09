//! FAR/DFARS clause database — risk levels and descriptions for ~50 common clauses.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RiskLevel {
    Green,
    Yellow,
    Red,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Green => "GREEN",
            RiskLevel::Yellow => "YELLOW",
            RiskLevel::Red => "RED",
        }
    }

    /// Numeric weight for scoring: GREEN=1, YELLOW=3, RED=10.
    pub fn weight(&self) -> u32 {
        match self {
            RiskLevel::Green => 1,
            RiskLevel::Yellow => 3,
            RiskLevel::Red => 10,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClauseInfo {
    pub number: String,
    pub title: String,
    pub risk: RiskLevel,
    pub description: String,
    pub recommendation: String,
}

/// Build and return the full clause database keyed by clause number (e.g. "52.212-4").
pub fn build_clause_db() -> HashMap<String, ClauseInfo> {
    let entries: &[(&str, &str, RiskLevel, &str, &str)] = &[
        // ── GREEN — standard protections / contractor-friendly ──────────────────
        (
            "52.212-4",
            "Contract Terms and Conditions — Commercial Products and Commercial Services",
            RiskLevel::Green,
            "Standard commercial-item terms. Balanced rights, dispute resolution, payment net-30.",
            "Baseline acceptable. Review for non-standard deviations.",
        ),
        (
            "52.233-1",
            "Disputes",
            RiskLevel::Green,
            "Mandates contractor continue performance during disputes. Provides claim rights.",
            "Favorable — preserves contractor right to submit certified claims.",
        ),
        (
            "52.249-2",
            "Termination for Convenience of the Government (Fixed-Price)",
            RiskLevel::Green,
            "Government may terminate; contractor recovers reasonable costs plus profit on completed work.",
            "Standard T4C. Ensure your accounting captures settlement-eligible costs.",
        ),
        (
            "52.232-1",
            "Payments",
            RiskLevel::Green,
            "Standard payment clause — Government pays within 30 days of proper invoice.",
            "Acceptable. Track invoice submission dates carefully.",
        ),
        (
            "52.232-25",
            "Prompt Payment",
            RiskLevel::Green,
            "Requires timely government payments; interest accrues automatically on late payments.",
            "Favorable. No action needed.",
        ),
        (
            "52.227-14",
            "Rights in Data — General",
            RiskLevel::Green,
            "Government receives unlimited rights in data first produced under the contract.",
            "Review if contract involves proprietary pre-existing data or software.",
        ),
        (
            "52.203-13",
            "Contractor Code of Business Ethics and Conduct",
            RiskLevel::Green,
            "Requires ethics program and hotline for contracts over $5.5M / 120 days.",
            "Standard compliance requirement. Confirm internal ethics program is in place.",
        ),
        (
            "52.222-26",
            "Equal Opportunity",
            RiskLevel::Green,
            "EEO obligations for contractors with 50+ employees or $50K+ contracts.",
            "Standard compliance. Ensure AAP is current if thresholds met.",
        ),
        (
            "52.204-9",
            "Personal Identity Verification of Contractor Personnel",
            RiskLevel::Green,
            "Requires PIV credentials for contractor employees who need access to federal facilities.",
            "Acceptable. Budget time for PIV issuance in project schedule.",
        ),
        (
            "52.225-13",
            "Restrictions on Certain Foreign Purchases",
            RiskLevel::Green,
            "Prohibits transactions with sanctioned countries/entities.",
            "Standard compliance. Verify supply chain has no sanctioned-entity exposure.",
        ),
        // ── YELLOW — moderate risk / watch closely ───────────────────────────────
        (
            "52.222-41",
            "Service Contract Labor Standards",
            RiskLevel::Yellow,
            "Requires payment of prevailing wages/fringe benefits set by Department of Labor wage determination.",
            "Obtain current WD before bidding. Non-compliance triggers debarment.",
        ),
        (
            "52.217-8",
            "Option to Extend Services",
            RiskLevel::Yellow,
            "Government may extend services up to 6 months at existing rates with 30-day notice.",
            "Price option periods carefully — government can hold you to current rates.",
        ),
        (
            "52.217-9",
            "Option to Extend the Term of the Contract",
            RiskLevel::Yellow,
            "Government may exercise priced option periods. Must be exercised before current period expires.",
            "Ensure option-year pricing accounts for wage escalation and inflation.",
        ),
        (
            "52.243-1",
            "Changes — Fixed-Price",
            RiskLevel::Yellow,
            "Allows unilateral government changes within general scope. Contractor may seek equitable adjustment.",
            "File REA promptly — 30-day constructive notice period applies. Track all change impacts.",
        ),
        (
            "52.215-10",
            "Price Reduction for Defective Certified Cost or Pricing Data",
            RiskLevel::Yellow,
            "Government can reduce contract price if certified cost data was defective.",
            "Ensure TINA compliance if contract exceeds $2M threshold.",
        ),
        (
            "52.222-50",
            "Combating Trafficking in Persons",
            RiskLevel::Yellow,
            "Prohibits trafficking in persons. Compliance plan required for contracts over $500K outside US.",
            "Review subcontracting chain. Compliance plan required for overseas performance.",
        ),
        (
            "52.216-7",
            "Allowable Cost and Payment",
            RiskLevel::Yellow,
            "Cost-reimbursement payment tied to allowable costs per FAR Part 31.",
            "Requires robust cost accounting system. DCAA audit exposure.",
        ),
        (
            "52.215-2",
            "Audit and Records — Negotiation",
            RiskLevel::Yellow,
            "Government has right to audit contractor records for 3 years post-completion.",
            "Maintain complete records. Establish document retention policy now.",
        ),
        (
            "52.228-5",
            "Insurance — Work on a Government Installation",
            RiskLevel::Yellow,
            "Requires specific insurance levels for work at government facilities.",
            "Verify coverage meets minimums before performance begins.",
        ),
        (
            "52.246-4",
            "Inspection of Services — Fixed-Price",
            RiskLevel::Yellow,
            "Government may inspect/reject services. Contractor remedies defects at own cost.",
            "Implement QC plan. Document acceptance of each deliverable.",
        ),
        (
            "52.244-6",
            "Subcontracts for Commercial Products and Commercial Services",
            RiskLevel::Yellow,
            "Requires flow-down of specific clauses to commercial-item subcontractors.",
            "Audit subcontract templates for required flow-downs.",
        ),
        (
            "52.219-14",
            "Limitations on Subcontracting",
            RiskLevel::Yellow,
            "Set-aside work must be self-performed at required percentages (e.g., 50% for services).",
            "Confirm self-performance capacity before bidding set-asides.",
        ),
        (
            "52.222-17",
            "Nondisplacement of Qualified Workers",
            RiskLevel::Yellow,
            "Successor contractor must offer qualified service workers right of first refusal.",
            "Budget for incumbent workforce absorption in transition plan.",
        ),
        (
            "52.204-21",
            "Basic Safeguarding of Covered Contractor Information Systems",
            RiskLevel::Yellow,
            "Requires 15 basic NIST SP 800-171 safeguarding requirements for covered information.",
            "Conduct gap assessment against NIST 800-171 before performance.",
        ),
        // ── RED — high risk / requires careful review ────────────────────────────
        (
            "52.249-8",
            "Default (Fixed-Price Supply and Service)",
            RiskLevel::Red,
            "Government may terminate for default, assess excess reprocurement costs, and pursue damages.",
            "CRITICAL: Termination for default triggers excess-cost liability. Negotiate cure-notice rights.",
        ),
        (
            "52.211-11",
            "Liquidated Damages — Supplies, Services, or Research and Development",
            RiskLevel::Red,
            "Pre-set damages per day of delay. Government withholds from payments automatically.",
            "CRITICAL: Negotiate LD cap. Document all government-caused delays to assert excusable delay.",
        ),
        (
            "52.211-12",
            "Liquidated Damages — Construction",
            RiskLevel::Red,
            "Daily LDs for late completion of construction. Can far exceed actual damages.",
            "CRITICAL: Verify LD rate is proportional. Consider delay contingency in price.",
        ),
        (
            "52.246-20",
            "Warranty of Services",
            RiskLevel::Red,
            "Contractor warrants services are performed in a workmanlike manner. Remediation at contractor cost.",
            "HIGH RISK: Define warranty period and scope precisely. Limit re-performance obligation.",
        ),
        (
            "52.228-1",
            "Bid Guarantee",
            RiskLevel::Red,
            "Failure to furnish required bid bond triggers forfeiture. Personal liability possible.",
            "CRITICAL: Obtain bid bond before submission. Confirm bond amount matches requirement.",
        ),
        (
            "52.232-27",
            "Prompt Payment for Construction Contracts",
            RiskLevel::Red,
            "Withholds retainage; subcontractor payment obligations flow down.",
            "HIGH RISK: Model cash flow with retainage withheld. Negotiate retainage reduction at 50%.",
        ),
        (
            "52.223-6",
            "Drug-Free Workplace",
            RiskLevel::Red,
            "Requires drug-free workplace program. Violations can trigger suspension/debarment.",
            "HIGH RISK: Implement written DFWP policy before performance.",
        ),
        (
            "52.203-7",
            "Anti-Kickback Procedures",
            RiskLevel::Red,
            "Requires reporting of kickbacks; violations are criminal.",
            "CRITICAL: Train all purchasing personnel. Implement anti-kickback reporting hotline.",
        ),
        (
            "52.209-6",
            "Protecting the Government's Interest When Subcontracting with Contractors Debarred, Suspended, or Proposed for Debarment",
            RiskLevel::Red,
            "Prohibits subcontracting with debarred/suspended entities. Contractor liable for violations.",
            "CRITICAL: Screen all subcontractors/suppliers against SAM.gov Exclusions before award.",
        ),
        (
            "252.204-7012",
            "Safeguarding Covered Defense Information and Cyber Incident Reporting",
            RiskLevel::Red,
            "DFARS: Requires NIST SP 800-171 compliance and 72-hour cyber incident reporting to DoD.",
            "CRITICAL: CMMC/800-171 gap assessment required. Non-compliance risks contract termination.",
        ),
        (
            "252.225-7001",
            "Buy American and Balance of Payments Program",
            RiskLevel::Red,
            "DFARS: End products must be domestic. Violation triggers price preference application.",
            "HIGH RISK: Audit entire supply chain for domestic content. Foreign product use needs waiver.",
        ),
        (
            "252.247-7023",
            "Transportation of Supplies by Sea",
            RiskLevel::Red,
            "DFARS: Ocean shipments must use US-flag vessels. Violations carry civil penalties.",
            "HIGH RISK: Coordinate with freight forwarder on US-flag compliance before shipment.",
        ),
        (
            "52.249-14",
            "Excusable Delays",
            RiskLevel::Red,
            "Only certain causes excuse delay (Acts of God, government acts, etc.). Burden on contractor.",
            "HIGH RISK: Document force majeure events immediately. Issue cure/show-cause responses promptly.",
        ),
        (
            "52.232-17",
            "Interest",
            RiskLevel::Red,
            "Contractor owes interest on government overpayments at Treasury rate.",
            "HIGH RISK: Reconcile all interim payments promptly to avoid accruing interest liability.",
        ),
        (
            "52.215-14",
            "Integrity of Unit Prices",
            RiskLevel::Red,
            "Prohibits spreading costs across line items (unbalanced bidding). Can trigger contract action.",
            "CRITICAL: Review pricing to ensure no unbalanced spread before submission.",
        ),
        (
            "52.203-3",
            "Gratuities",
            RiskLevel::Red,
            "Government may terminate contract if contractor offered gratuities to influence award or oversight.",
            "CRITICAL: Document all government employee contacts. No gifts, meals, or entertainment.",
        ),
    ];

    let mut db = HashMap::with_capacity(entries.len());
    for (number, title, risk, description, recommendation) in entries {
        db.insert(
            number.to_string(),
            ClauseInfo {
                number: number.to_string(),
                title: title.to_string(),
                risk: risk.clone(),
                description: description.to_string(),
                recommendation: recommendation.to_string(),
            },
        );
    }
    db
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_has_expected_size() {
        let db = build_clause_db();
        assert!(db.len() >= 40, "expected >= 40 clauses, got {}", db.len());
    }

    #[test]
    fn default_clause_lookup() {
        let db = build_clause_db();
        let c = db.get("52.249-8").expect("52.249-8 should be in DB");
        assert_eq!(c.risk, RiskLevel::Red);
    }

    #[test]
    fn green_clause_weight_is_lowest() {
        assert!(RiskLevel::Green.weight() < RiskLevel::Yellow.weight());
        assert!(RiskLevel::Yellow.weight() < RiskLevel::Red.weight());
    }

    #[test]
    fn dfars_clause_present() {
        let db = build_clause_db();
        assert!(db.contains_key("252.204-7012"), "DFARS 252.204-7012 missing");
    }

    #[test]
    fn risk_level_as_str() {
        assert_eq!(RiskLevel::Green.as_str(), "GREEN");
        assert_eq!(RiskLevel::Yellow.as_str(), "YELLOW");
        assert_eq!(RiskLevel::Red.as_str(), "RED");
    }
}
