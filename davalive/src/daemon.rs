//! DAVA's heartbeat daemon — runs all 5 bus connectors in a background loop.
//!
//! Call `start_daemon()` to block forever. Each tick runs all connectors once
//! and logs any events processed to stderr.

use hoags_core::bus::{BusRunner, EventBus};

/// Start the daemon and block until the process is killed.
pub fn start_daemon() {
    println!("DAVA LIVE — Starting heartbeat daemon");
    println!("Connecting to event bus...");

    let bus = EventBus::open_default().expect("Failed to open bus");
    let runner = BusRunner::new(bus).with_interval(std::time::Duration::from_secs(10));

    println!("5 connectors active. Heartbeat every 10s.");
    println!("Press Ctrl+C to stop.");

    runner.run_loop(); // blocks forever
}

/// Run one heartbeat pass (useful for `davalive pulse`).
/// Returns a vector of (connector_name, events_processed, knowledge_shared) tuples.
pub fn run_once() -> Vec<ConnectorSummary> {
    let bus = EventBus::open_default().expect("Failed to open bus");
    let runner = BusRunner::new(bus);
    let results = runner.run_once();
    results
        .into_iter()
        .map(|(name, r)| ConnectorSummary {
            name,
            events_processed: r.events_processed,
            knowledge_shared: r.knowledge_shared,
            memory_updates: r.memory_updates,
        })
        .collect()
}

#[derive(Debug)]
pub struct ConnectorSummary {
    pub name: String,
    pub events_processed: usize,
    pub knowledge_shared: usize,
    pub memory_updates: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoags_core::bus::EventBus;
    use tempfile::NamedTempFile;

    #[test]
    fn test_run_once_returns_five_connectors() {
        // Publish an event so connectors have something to process
        let tmp = NamedTempFile::new().unwrap();
        let bus = EventBus::open(tmp.path()).unwrap();
        bus.publish("pdffill", "pdffill.form_detected", r#"{"template":"SF1449"}"#);
        let runner = hoags_core::bus::BusRunner::new(bus);
        let results = runner.run_once();
        assert_eq!(results.len(), 5, "Should always run exactly 5 connectors");
    }

    #[test]
    fn test_run_once_no_events_still_five_connectors() {
        let tmp = NamedTempFile::new().unwrap();
        let bus = EventBus::open(tmp.path()).unwrap();
        let runner = hoags_core::bus::BusRunner::new(bus);
        let results = runner.run_once();
        assert_eq!(results.len(), 5);
        let total: usize = results.iter().map(|(_, r)| r.events_processed).sum();
        // With no events, connectors still run but process 0 events
        assert_eq!(total, 0);
    }

    #[test]
    fn test_connector_summary_struct() {
        let s = ConnectorSummary {
            name: "connector_bid".to_string(),
            events_processed: 3,
            knowledge_shared: 2,
            memory_updates: 1,
        };
        assert_eq!(s.name, "connector_bid");
        assert_eq!(s.events_processed, 3);
        assert_eq!(s.knowledge_shared, 2);
        assert_eq!(s.memory_updates, 1);
    }
}
