//! # Container — Typed Response Wrapper
//!
//! A container holds cargo during transport — the MCP response wrapper.
//! Like a shipping container: reusable, standardized, with a packing list
//! that describes contents without opening the box.
//!
//! Empty containers (tool schemas without responses) are documented as cargo.
//! When tools return data, the container becomes "containerized cargo."

use crate::cargo::Cargo;
use crate::route::FreightRoute;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Packing list — metadata about container contents without inspecting the cargo.
///
/// Like a shipping manifest: you know how many items, their type, total weight,
/// and whether special handling is required — without unpacking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackingList {
    /// Number of cargo items in this container
    pub item_count: usize,
    /// Type name for the cargo (for routing without deserialization)
    pub cargo_type: String,
    /// Serialized size in bytes (transport weight)
    pub total_weight_bytes: usize,
    /// Whether the cargo contains PII or sensitive data requiring special handling
    pub hazmat: bool,
}

impl fmt::Display for PackingList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} × {} ({} bytes{})",
            self.item_count,
            self.cargo_type,
            self.total_weight_bytes,
            if self.hazmat { ", HAZMAT" } else { "" }
        )
    }
}

/// A container holds cargo during transport.
///
/// Generic over the cargo type `C`. The container adds transport metadata
/// (packing list, route) without modifying the cargo itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "C: Serialize + serde::de::DeserializeOwned")]
pub struct Container<C: Cargo> {
    /// The cargo items being transported
    cargo: Vec<C>,
    /// Metadata about the contents
    packing_list: PackingList,
    /// The planned route for this container
    route: FreightRoute,
}

impl<C: Cargo> Container<C> {
    /// Create a container with cargo and route.
    ///
    /// The packing list is generated automatically from the cargo.
    #[must_use]
    pub fn pack(cargo: Vec<C>, route: FreightRoute, cargo_type: impl Into<String>) -> Self {
        let packing_list = PackingList {
            item_count: cargo.len(),
            cargo_type: cargo_type.into(),
            total_weight_bytes: 0, // Caller can set after serialization
            hazmat: false,
        };
        Self {
            cargo,
            packing_list,
            route,
        }
    }

    /// Create a container with explicit packing list.
    #[must_use]
    pub fn pack_with_manifest(
        cargo: Vec<C>,
        packing_list: PackingList,
        route: FreightRoute,
    ) -> Self {
        Self {
            cargo,
            packing_list,
            route,
        }
    }

    /// Access the cargo items.
    #[must_use]
    pub fn cargo(&self) -> &[C] {
        &self.cargo
    }

    /// Consume the container and return the cargo.
    #[must_use]
    pub fn unpack(self) -> Vec<C> {
        self.cargo
    }

    /// Access the packing list.
    #[must_use]
    pub fn packing_list(&self) -> &PackingList {
        &self.packing_list
    }

    /// Access the route.
    #[must_use]
    pub fn route(&self) -> &FreightRoute {
        &self.route
    }

    /// Mutable access to the route (for perishability upgrades during transit).
    pub fn route_mut(&mut self) -> &mut FreightRoute {
        &mut self.route
    }

    /// Whether this container is empty (no cargo items).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cargo.is_empty()
    }

    /// Number of cargo items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cargo.len()
    }

    /// Mark this container as containing hazardous materials (PII/sensitive data).
    pub fn mark_hazmat(&mut self) {
        self.packing_list.hazmat = true;
    }

    /// Update the total weight after serialization.
    pub fn set_weight(&mut self, bytes: usize) {
        self.packing_list.total_weight_bytes = bytes;
    }
}

impl<C: Cargo> fmt::Display for Container<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Container[{}] via {}", self.packing_list, self.route)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Container tests require a Cargo impl — see integration tests
    // in cargo.rs where we define a test cargo type.

    #[test]
    fn test_packing_list_display() {
        let pl = PackingList {
            item_count: 50,
            cargo_type: "FaersCargo".to_string(),
            total_weight_bytes: 4096,
            hazmat: false,
        };
        assert_eq!(pl.to_string(), "50 × FaersCargo (4096 bytes)");
    }

    #[test]
    fn test_packing_list_hazmat_display() {
        let pl = PackingList {
            item_count: 3,
            cargo_type: "IcsrCargo".to_string(),
            total_weight_bytes: 1024,
            hazmat: true,
        };
        assert_eq!(pl.to_string(), "3 × IcsrCargo (1024 bytes, HAZMAT)");
    }
}
