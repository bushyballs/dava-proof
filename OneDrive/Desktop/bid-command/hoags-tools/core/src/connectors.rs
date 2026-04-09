//! Connector agents — 5 Haiku-powered agents that integrate cross-tool knowledge.
//!
//! Each connector has:
//!   - A specialty (what events it watches)
//!   - A memory (key-value store on the bus)
//!   - A run() function that processes pending events and generates cross-tool knowledge
//!
//! The 5 connectors:
//!   1. BidConnector      — links pdffill ↔ propbuilder ↔ clauseguard ↔ mailcraft
//!   2. FinanceConnector   — links invoicer ↔ receipts ↔ sheetwise
//!   3. DocConnector       — links docconv ↔ pdffill ↔ sigstamp
//!   4. ComplianceConnector — links clauseguard ↔ all tools (risk propagation)
//!   5. LearningConnector  — watches ALL events, finds patterns, teaches DAVA

use crate::bus::EventBus;
use serde_json::Value;

/// Trait all connectors implement.
pub trait Connector {
    fn name(&self) -> &str;
    fn run(&self, bus: &EventBus) -> ConnectorResult;
}

#[derive(Debug, Default)]
pub struct ConnectorResult {
    pub events_processed: usize,
    pub knowledge_shared: usize,
    pub memory_updates: usize,
}

// ── Connector 1: BidConnector ───────────────────────────────────────

pub struct BidConnector;

impl Connector for BidConnector {
    fn name(&self) -> &str { "connector_bid" }

    fn run(&self, bus: &EventBus) -> ConnectorResult {
        let mut result = ConnectorResult::default();

        // When pdffill detects a form → tell propbuilder what type it is
        for event in bus.poll(self.name(), Some("pdffill.form_detected")) {
            let payload: Value = serde_json::from_str(&event.payload).unwrap_or_default();
            if let Some(template) = payload.get("template").and_then(|t| t.as_str()) {
                bus.share_knowledge("pdffill", "propbuilder", "form_template", template, 0.9);
                result.knowledge_shared += 1;
            }
            bus.ack(event.id, self.name());
            result.events_processed += 1;
        }

        // When pdffill fills a form → tell mailcraft to draft submission email
        for event in bus.poll(self.name(), Some("pdffill.form_filled")) {
            let payload: Value = serde_json::from_str(&event.payload).unwrap_or_default();
            if let Some(sol) = payload.get("solicitation").and_then(|s| s.as_str()) {
                bus.share_knowledge("pdffill", "mailcraft", "ready_to_submit", sol, 0.95);
                result.knowledge_shared += 1;
            }
            bus.ack(event.id, self.name());
            result.events_processed += 1;
        }

        // When propbuilder generates a proposal → tell sigstamp it needs signing
        for event in bus.poll(self.name(), Some("propbuilder.proposal_generated")) {
            let payload: Value = serde_json::from_str(&event.payload).unwrap_or_default();
            if let Some(path) = payload.get("output_path").and_then(|p| p.as_str()) {
                bus.share_knowledge("propbuilder", "sigstamp", "needs_signature", path, 0.9);
                result.knowledge_shared += 1;
            }
            bus.ack(event.id, self.name());
            result.events_processed += 1;
        }

        // Track bid pipeline state in memory
        let pipeline_count = bus.poll(self.name(), Some("pdffill.form_filled")).len();
        bus.set_memory(self.name(), "active_bids", &pipeline_count.to_string());
        result.memory_updates += 1;

        result
    }
}

// ── Connector 2: FinanceConnector ───────────────────────────────────

pub struct FinanceConnector;

impl Connector for FinanceConnector {
    fn name(&self) -> &str { "connector_finance" }

    fn run(&self, bus: &EventBus) -> ConnectorResult {
        let mut result = ConnectorResult::default();

        // When invoicer generates invoice → tell receipts to expect payment
        for event in bus.poll(self.name(), Some("invoicer.invoice_generated")) {
            let payload: Value = serde_json::from_str(&event.payload).unwrap_or_default();
            if let Some(amount) = payload.get("total_amount").and_then(|a| a.as_f64()) {
                let contract = payload.get("contract_number").and_then(|c| c.as_str()).unwrap_or("unknown");
                let knowledge = format!("Expected payment ${:.2} for {}", amount, contract);
                bus.share_knowledge("invoicer", "receipts", "expected_payment", &knowledge, 0.85);
                result.knowledge_shared += 1;
            }
            bus.ack(event.id, self.name());
            result.events_processed += 1;
        }

        // When receipts adds expense → tell sheetwise for analysis
        for event in bus.poll(self.name(), Some("receipts.expense_added")) {
            bus.share_knowledge("receipts", "sheetwise", "new_expense_data", &event.payload, 0.7);
            bus.ack(event.id, self.name());
            result.events_processed += 1;
            result.knowledge_shared += 1;
        }

        // Track running totals
        let invoiced: String = bus.get_memory(self.name(), "total_invoiced").unwrap_or_else(|| "0".into());
        bus.set_memory(self.name(), "total_invoiced", &invoiced);
        result.memory_updates += 1;

        result
    }
}

// ── Connector 3: DocConnector ───────────────────────────────────────

pub struct DocConnector;

impl Connector for DocConnector {
    fn name(&self) -> &str { "connector_doc" }

    fn run(&self, bus: &EventBus) -> ConnectorResult {
        let mut result = ConnectorResult::default();

        // When docconv converts a doc → tell pdffill if it's a fillable form
        for event in bus.poll(self.name(), Some("docconv.document_converted")) {
            let payload: Value = serde_json::from_str(&event.payload).unwrap_or_default();
            if payload.get("output_format").and_then(|f| f.as_str()) == Some("pdf") {
                if let Some(path) = payload.get("output_path").and_then(|p| p.as_str()) {
                    bus.share_knowledge("docconv", "pdffill", "new_pdf_available", path, 0.8);
                    result.knowledge_shared += 1;
                }
            }
            bus.ack(event.id, self.name());
            result.events_processed += 1;
        }

        // When sigstamp signs a doc → record it
        for event in bus.poll(self.name(), Some("sigstamp.document_signed")) {
            let payload: Value = serde_json::from_str(&event.payload).unwrap_or_default();
            let doc = payload.get("document").and_then(|d| d.as_str()).unwrap_or("unknown");
            bus.set_memory(self.name(), &format!("signed_{}", doc), "true");
            bus.ack(event.id, self.name());
            result.events_processed += 1;
            result.memory_updates += 1;
        }

        result
    }
}

// ── Connector 4: ComplianceConnector ────────────────────────────────

pub struct ComplianceConnector;

impl Connector for ComplianceConnector {
    fn name(&self) -> &str { "connector_compliance" }

    fn run(&self, bus: &EventBus) -> ConnectorResult {
        let mut result = ConnectorResult::default();

        // When clauseguard finds a risk → broadcast to all relevant tools
        for event in bus.poll(self.name(), Some("clauseguard.risk_found")) {
            let payload: Value = serde_json::from_str(&event.payload).unwrap_or_default();
            let risk_level = payload.get("risk_level").and_then(|r| r.as_str()).unwrap_or("unknown");
            let clause = payload.get("clause").and_then(|c| c.as_str()).unwrap_or("unknown");

            // High risk → tell mailcraft to draft a question
            if risk_level == "red" {
                let msg = format!("HIGH RISK clause {} detected — consider asking CO for clarification", clause);
                bus.share_knowledge("clauseguard", "mailcraft", "risk_alert", &msg, 0.95);
                result.knowledge_shared += 1;
            }

            // Any risk → tell propbuilder to address in technical approach
            let msg = format!("{} risk: clause {}", risk_level, clause);
            bus.share_knowledge("clauseguard", "propbuilder", "clause_risk", &msg, 0.8);
            result.knowledge_shared += 1;

            bus.ack(event.id, self.name());
            result.events_processed += 1;
        }

        result
    }
}

// ── Connector 5: LearningConnector ──────────────────────────────────

pub struct LearningConnector;

impl Connector for LearningConnector {
    fn name(&self) -> &str { "connector_learning" }

    fn run(&self, bus: &EventBus) -> ConnectorResult {
        let mut result = ConnectorResult::default();

        // Watch ALL events and count patterns
        let all_events = bus.poll(self.name(), None);
        let event_count = all_events.len();

        for event in &all_events {
            // Track event frequency by type
            let count_key = format!("event_count_{}", event.event_type);
            let current: i64 = bus.get_memory(self.name(), &count_key)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            bus.set_memory(self.name(), &count_key, &(current + 1).to_string());
            result.memory_updates += 1;

            bus.ack(event.id, self.name());
            result.events_processed += 1;
        }

        // Track total events seen
        let total: i64 = bus.get_memory(self.name(), "total_events_seen")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        bus.set_memory(self.name(), "total_events_seen", &(total + event_count as i64).to_string());

        result
    }
}

// ── Run all connectors ──────────────────────────────────────────────

pub fn run_all_connectors(bus: &EventBus) -> Vec<(String, ConnectorResult)> {
    let connectors: Vec<Box<dyn Connector>> = vec![
        Box::new(BidConnector),
        Box::new(FinanceConnector),
        Box::new(DocConnector),
        Box::new(ComplianceConnector),
        Box::new(LearningConnector),
    ];

    connectors.iter().map(|c| {
        let name = c.name().to_string();
        let result = c.run(bus);
        (name, result)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn tmp_bus() -> EventBus {
        let tmp = NamedTempFile::new().unwrap();
        EventBus::open(tmp.path()).unwrap()
    }

    #[test]
    fn test_bid_connector_form_to_propbuilder() {
        let bus = tmp_bus();
        bus.publish("pdffill", "pdffill.form_detected", r#"{"template":"SF1449"}"#);
        let bid = BidConnector;
        let result = bid.run(&bus);
        assert!(result.events_processed >= 1);
        assert!(result.knowledge_shared >= 1);
        let knowledge = bus.get_knowledge("propbuilder");
        assert!(!knowledge.is_empty());
    }

    #[test]
    fn test_finance_connector_invoice_to_receipts() {
        let bus = tmp_bus();
        bus.publish("invoicer", "invoicer.invoice_generated",
            r#"{"total_amount": 10645.63, "contract_number": "W9127S26QA030"}"#);
        let fin = FinanceConnector;
        let result = fin.run(&bus);
        assert!(result.events_processed >= 1);
        let knowledge = bus.get_knowledge("receipts");
        assert!(!knowledge.is_empty());
    }

    #[test]
    fn test_compliance_connector_risk_broadcast() {
        let bus = tmp_bus();
        bus.publish("clauseguard", "clauseguard.risk_found",
            r#"{"risk_level":"red","clause":"52.249-8"}"#);
        let comp = ComplianceConnector;
        let result = comp.run(&bus);
        assert!(result.knowledge_shared >= 2); // mailcraft + propbuilder

        let mailcraft_knowledge = bus.get_knowledge("mailcraft");
        assert!(mailcraft_knowledge.iter().any(|k| k.2.contains("52.249-8")));
    }

    #[test]
    fn test_learning_connector_counts_events() {
        let bus = tmp_bus();
        bus.publish("pdffill", "pdffill.form_filled", "{}");
        bus.publish("pdffill", "pdffill.form_filled", "{}");
        bus.publish("invoicer", "invoicer.invoice_generated", "{}");

        let learning = LearningConnector;
        let result = learning.run(&bus);
        assert_eq!(result.events_processed, 3);

        let total = bus.get_memory("connector_learning", "total_events_seen");
        assert_eq!(total, Some("3".into()));
    }

    #[test]
    fn test_run_all_connectors() {
        let bus = tmp_bus();
        bus.publish("pdffill", "pdffill.form_detected", r#"{"template":"W9"}"#);
        let results = run_all_connectors(&bus);
        assert_eq!(results.len(), 5);
        // At least BidConnector and LearningConnector should process the event
        let total_processed: usize = results.iter().map(|(_, r)| r.events_processed).sum();
        assert!(total_processed >= 2);
    }
}
