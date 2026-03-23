//! # Cargo Trait — Typed Payload in Transit
//!
//! The core abstraction: every PV domain object that transits through the
//! system implements `Cargo`. The trait captures what logistics captures —
//! manifest metadata independent of conveyance.
//!
//! Cargo is never "at rest." Even in storage, it has a destination and
//! perishability window. The cold-chain principle: perishable inventory
//! is always in transit, even when held in storage.

use crate::destination::Destination;
use crate::perishability::Perishability;
use crate::provenance::Provenance;
use crate::stamp::{CustodyChain, StationStamp};
use serde::{Deserialize, Serialize};

/// A typed payload in transit toward a safety decision.
///
/// Every implementor carries:
/// - **Provenance**: where it came from (data source, query, timestamp)
/// - **Destination**: what safety decision it's moving toward
/// - **Perishability**: regulatory reporting deadline (can upgrade mid-transit)
/// - **Custody chain**: ordered list of stations that handled it
///
/// # Implementors
///
/// Domain crates (e.g., `nexcore-vigilance`) implement this for their types:
/// - `FaersCargo` — FAERS adverse event reports
/// - `SignalCargo` — PRR/ROR/IC/EBGM signal scores
/// - `CausalityCargo` — Naranjo/WHO-UMC causality assessments
/// - `IcsrCargo` — Individual Case Safety Reports
pub trait Cargo: Sized + Send + Sync {
    /// The domain type this cargo carries.
    type Payload;

    /// Where this cargo originated.
    fn provenance(&self) -> &Provenance;

    /// What safety decision this cargo is moving toward.
    fn destination(&self) -> Destination;

    /// Regulatory reporting deadline — the cold-chain expiry.
    fn perishability(&self) -> Perishability;

    /// Stations that have handled this cargo (chain of custody).
    fn custody_chain(&self) -> &CustodyChain;

    /// The actual domain payload.
    fn payload(&self) -> &Self::Payload;

    /// Stamp this cargo as having passed through a station.
    fn stamp(&mut self, stamp: StationStamp);

    /// Upgrade perishability to a more urgent level.
    ///
    /// Called when transit processing reveals higher urgency
    /// (e.g., signal detection finds a strong signal, upgrading
    /// from Periodic to Prompt).
    fn upgrade_perishability(&mut self, new: Perishability);
}

/// A simple cargo wrapper for any serializable payload.
///
/// Use this when you need a quick `Cargo` implementation without
/// defining a full domain-specific type. For production domain types,
/// implement `Cargo` directly on your type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "T: Serialize + serde::de::DeserializeOwned")]
pub struct SimpleCargo<T: Send + Sync> {
    provenance: Provenance,
    destination: Destination,
    perishability: Perishability,
    custody: CustodyChain,
    payload: T,
}

impl<T: Send + Sync> SimpleCargo<T> {
    /// Create a new simple cargo.
    #[must_use]
    pub fn new(
        payload: T,
        provenance: Provenance,
        destination: Destination,
        perishability: Perishability,
    ) -> Self {
        Self {
            provenance,
            destination,
            perishability,
            custody: CustodyChain::new(),
            payload,
        }
    }
}

impl<T: Send + Sync + Serialize + for<'de> Deserialize<'de>> Cargo for SimpleCargo<T> {
    type Payload = T;

    fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    fn destination(&self) -> Destination {
        self.destination
    }

    fn perishability(&self) -> Perishability {
        self.perishability
    }

    fn custody_chain(&self) -> &CustodyChain {
        &self.custody
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn stamp(&mut self, stamp: StationStamp) {
        self.custody.stamp(stamp);
    }

    fn upgrade_perishability(&mut self, new: Perishability) {
        self.perishability = self.perishability.upgrade(new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::{DataSource, QueryParams};

    fn test_provenance() -> Provenance {
        let mut query = QueryParams::empty();
        query.insert("drug", "metformin");
        Provenance::new(DataSource::Faers, query, 1709856000, 0.98)
    }

    #[test]
    fn test_simple_cargo_creation() {
        let cargo = SimpleCargo::new(
            "test payload".to_string(),
            test_provenance(),
            Destination::SignalDetection,
            Perishability::Periodic,
        );

        assert_eq!(cargo.destination(), Destination::SignalDetection);
        assert_eq!(cargo.perishability(), Perishability::Periodic);
        assert_eq!(cargo.custody_chain().hop_count(), 0);
        assert_eq!(cargo.payload(), "test payload");
    }

    #[test]
    fn test_stamping() {
        let mut cargo = SimpleCargo::new(
            42u64,
            test_provenance(),
            Destination::SignalDetection,
            Perishability::Periodic,
        );

        cargo.stamp(StationStamp::new("station-a", "process", 1000, 0.95));
        cargo.stamp(StationStamp::new("station-b", "analyze", 1001, 0.93));

        assert_eq!(cargo.custody_chain().hop_count(), 2);
        let expected_fidelity = 0.95 * 0.93;
        assert!((cargo.custody_chain().cumulative_fidelity() - expected_fidelity).abs() < 1e-10);
    }

    #[test]
    fn test_perishability_upgrade_during_transit() {
        let mut cargo = SimpleCargo::new(
            "event_data".to_string(),
            test_provenance(),
            Destination::SignalDetection,
            Perishability::Periodic,
        );

        // Signal detected — upgrade to Prompt
        cargo.upgrade_perishability(Perishability::PROMPT_90);
        assert_eq!(cargo.perishability(), Perishability::PROMPT_90);

        // Fatal case confirmed — upgrade to Expedited
        cargo.upgrade_perishability(Perishability::EXPEDITED_15);
        assert_eq!(cargo.perishability(), Perishability::EXPEDITED_15);

        // Cannot downgrade
        cargo.upgrade_perishability(Perishability::Periodic);
        assert_eq!(cargo.perishability(), Perishability::EXPEDITED_15);
    }

    #[test]
    fn test_full_transit_scenario() {
        // Simulate: FAERS query → signal detection → causality assessment
        let mut cargo = SimpleCargo::new(
            "metformin + lactic acidosis".to_string(),
            test_provenance(),
            Destination::SignalDetection,
            Perishability::Periodic,
        );

        // Station 1: openFDA ingest
        cargo.stamp(StationStamp::new(
            "nexvigilant-station::openfda",
            "search_adverse_events",
            1709856100,
            0.98,
        ));

        // Station 2: Signal detection — finds strong signal, upgrade perishability
        cargo.stamp(StationStamp::new(
            "signal-pipeline::detect",
            "prr_compute",
            1709856200,
            0.93,
        ));
        cargo.upgrade_perishability(Perishability::PROMPT_90);

        // Station 3: Causality assessment
        cargo.stamp(StationStamp::new(
            "microgram::naranjo-quick",
            "naranjo_score",
            1709856300,
            0.95,
        ));

        // Verify chain
        assert_eq!(cargo.custody_chain().hop_count(), 3);
        assert_eq!(cargo.perishability(), Perishability::PROMPT_90);

        let f_total = cargo.custody_chain().cumulative_fidelity();
        let expected = 0.98 * 0.93 * 0.95;
        assert!((f_total - expected).abs() < 1e-10);
        assert!(cargo.custody_chain().meets_safety_threshold());
    }
}
