//! # nexcore-cargo — Typed Transport for PV Rails
//!
//! Separates **cargo** (typed domain payload) from **freight** (conveyance protocol).
//!
//! In transportation: cargo refers to goods, freight refers to conveyance.
//! In NexVigilant: cargo is PV domain data (signals, cases, assessments),
//! freight is the MCP/microgram transport that moves it between stations.
//!
//! ## Core Concepts
//!
//! | Concept | Type | What It Does |
//! |---------|------|-------------|
//! | Cargo | `Cargo` trait | Typed payload with provenance, destination, perishability |
//! | Provenance | `Provenance` | Where cargo originated (data source, query, confidence) |
//! | Destination | `Destination` | What safety decision cargo moves toward |
//! | Perishability | `Perishability` | Reporting deadline — the cold-chain temperature |
//! | Station Stamp | `StationStamp` | Chain of custody record per processing hop |
//! | Container | `Container<C>` | Transport wrapper with packing list |
//! | Freight Route | `FreightRoute` | Planned path through stations (bill of lading) |
//!
//! ## Cold-Chain Principle
//!
//! Perishability can **upgrade** during transit but never downgrade. A routine
//! FAERS query starts as `Periodic`. Signal detection upgrades it to `Prompt(90)`.
//! Fatal causality assessment upgrades it to `Expedited(15)`. The cargo's urgency
//! is discovered during transit, not known at loading.
//!
//! ## Layer Position
//!
//! Foundation layer — depends on `serde` only. Consumed by domain crates
//! (`nexcore-vigilance`), orchestration (`nexcore-signal-pipeline`), and
//! service (`nexcore-mcp`).

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod cargo;
pub mod container;
pub mod destination;
pub mod perishability;
pub mod provenance;
pub mod route;
pub mod stamp;

// Re-export core types at crate root
pub use cargo::{Cargo, SimpleCargo};
pub use container::{Container, PackingList};
pub use destination::Destination;
pub use perishability::Perishability;
pub use provenance::{DataSource, Provenance, QueryParams};
pub use route::{FreightRoute, Priority, Waypoint};
pub use stamp::{CustodyChain, StationStamp};
