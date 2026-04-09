#[cfg(test)]
mod integration_tests {
    use crate::bus::EventBus;
    use crate::connectors::*;
    use tempfile::NamedTempFile;

    fn fresh_bus() -> (NamedTempFile, EventBus) {
        let tmp = NamedTempFile::new().unwrap();
        let bus = EventBus::open(tmp.path()).unwrap();
        (tmp, bus)
    }

    // Test the full bid pipeline flow
    #[test]
    fn test_bid_pipeline_flow() {
        let (_tmp, bus) = fresh_bus();

        // 1. pdffill detects a form
        bus.publish("pdffill", "pdffill.form_detected", r#"{"template":"SF1449"}"#);

        // 2. Run BidConnector
        let bid = BidConnector;
        bid.run(&bus);

        // 3. propbuilder should have received knowledge
        let knowledge = bus.get_knowledge("propbuilder");
        assert!(!knowledge.is_empty());
        assert!(knowledge.iter().any(|k| k.2.contains("SF1449")));
    }

    // Test finance flow
    #[test]
    fn test_finance_flow() {
        let (_tmp, bus) = fresh_bus();

        bus.publish("invoicer", "invoicer.invoice_generated",
            r#"{"total_amount": 10000.0, "contract_number": "W9127S"}"#);

        let fin = FinanceConnector;
        fin.run(&bus);

        let knowledge = bus.get_knowledge("receipts");
        assert!(!knowledge.is_empty());
    }

    // Test compliance risk broadcast
    #[test]
    fn test_compliance_broadcast() {
        let (_tmp, bus) = fresh_bus();

        bus.publish("clauseguard", "clauseguard.risk_found",
            r#"{"risk_level":"red","clause":"52.249-8"}"#);

        let comp = ComplianceConnector;
        comp.run(&bus);

        // Both mailcraft and propbuilder should get knowledge
        let mail_k = bus.get_knowledge("mailcraft");
        let prop_k = bus.get_knowledge("propbuilder");
        assert!(!mail_k.is_empty());
        assert!(!prop_k.is_empty());
    }

    // Test learning connector counts all events
    #[test]
    fn test_learning_counts_all() {
        let (_tmp, bus) = fresh_bus();

        bus.publish("pdffill", "pdffill.form_filled", "{}");
        bus.publish("invoicer", "invoicer.invoice_generated", "{}");
        bus.publish("sigstamp", "sigstamp.document_signed", "{}");

        let learning = LearningConnector;
        learning.run(&bus);

        let total = bus.get_memory("connector_learning", "total_events_seen");
        assert_eq!(total, Some("3".into()));
    }

    // Test full run_all_connectors
    #[test]
    fn test_full_connector_pass() {
        let (_tmp, bus) = fresh_bus();

        bus.publish("pdffill", "pdffill.form_detected", r#"{"template":"W-9"}"#);
        bus.publish("clauseguard", "clauseguard.risk_found", r#"{"risk_level":"yellow","clause":"52.222-41"}"#);

        let results = run_all_connectors(&bus);
        assert_eq!(results.len(), 5);

        let total_processed: usize = results.iter().map(|(_, r)| r.events_processed).sum();
        assert!(total_processed >= 4); // each event processed by specific + learning connector
    }

    // Test doc connector routes PDF conversions
    #[test]
    fn test_doc_connector_routes_pdf() {
        let (_tmp, bus) = fresh_bus();

        bus.publish("docconv", "docconv.document_converted",
            r#"{"output_format":"pdf","output_path":"/tmp/out.pdf"}"#);

        let doc = DocConnector;
        doc.run(&bus);

        let knowledge = bus.get_knowledge("pdffill");
        assert!(!knowledge.is_empty());
    }

    // Test bus purge preserves recent events
    #[test]
    fn test_purge_preserves_recent() {
        let (_tmp, bus) = fresh_bus();

        bus.publish("pdffill", "test", "{}");
        let purged = bus.purge_old(1); // purge older than 1 day
        assert_eq!(purged, 0); // just published, shouldn't be purged

        let stats = bus.stats();
        assert_eq!(stats.total_events, 1);
    }

    // Test event_count_by_tool
    #[test]
    fn test_event_count_by_tool() {
        let (_tmp, bus) = fresh_bus();

        bus.publish("pdffill", "a", "{}");
        bus.publish("pdffill", "b", "{}");
        bus.publish("invoicer", "c", "{}");

        let counts = bus.event_count_by_tool();
        assert_eq!(counts.get("pdffill"), Some(&2));
        assert_eq!(counts.get("invoicer"), Some(&1));
    }

    // Test pending_for counts correctly
    #[test]
    fn test_pending_for() {
        let (_tmp, bus) = fresh_bus();

        bus.publish("pdffill", "test", "{}");
        assert_eq!(bus.pending_for("clauseguard"), 1);

        let events = bus.poll("clauseguard", None);
        bus.ack(events[0].id, "clauseguard");
        assert_eq!(bus.pending_for("clauseguard"), 0);
    }

    // Test export_json includes all tables
    #[test]
    fn test_export_json_complete() {
        let (_tmp, bus) = fresh_bus();

        bus.publish("pdffill", "test", r#"{"key":"value"}"#);
        bus.share_knowledge("a", "b", "type", "content", 0.9);
        bus.set_memory("conn", "key", "val");

        let json = bus.export_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed["events"].as_array().unwrap().len() >= 1);
        assert!(parsed["knowledge"].as_array().unwrap().len() >= 1);
        assert!(parsed["memory"].as_array().unwrap().len() >= 1);
    }
}
