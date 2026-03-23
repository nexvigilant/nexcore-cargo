//! # Perishability — The Cold-Chain Temperature of PV Cargo
//!
//! In logistics, perishable goods require cold-chain management: temperature
//! monitoring, expiry tracking, and priority routing. In PV, the "temperature"
//! is the regulatory reporting deadline.
//!
//! Key insight: **perishability can change during transit.** A routine FAERS
//! query result starts as `Periodic`. When signal detection finds PRR > 2.0,
//! it upgrades to `Prompt(90)`. When causality assessment returns `Certain`
//! for a fatal case, it upgrades again to `Expedited(15)`.
//!
//! The cargo's urgency is discovered during transit, not known at loading.
//! This maps to ICH E2D Section 3.1: "Day 0 is the date the MAH first
//! becomes aware of information that meets minimum criteria for reporting."

use serde::{Deserialize, Serialize};
use std::fmt;

/// Reporting deadline classification — the cold-chain temperature.
///
/// Ordered by urgency: `Expedited` > `Prompt` > `Periodic` > `NonPerishable`.
/// Perishability can only upgrade (become more urgent), never downgrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Perishability {
    /// Fatal or life-threatening cases. Typically 15 calendar days from Day 0.
    /// ICH E2D: "serious, unexpected adverse reactions that are fatal or
    /// life-threatening" require expedited reporting.
    Expedited {
        /// Calendar days from Day 0 awareness
        deadline_days: u16,
    },
    /// Serious, non-fatal cases. Typically 90 calendar days.
    /// ICH E2D: non-expedited serious cases reported within periodic timeframes.
    Prompt {
        /// Calendar days from Day 0 awareness
        deadline_days: u16,
    },
    /// Aggregate reporting in next PSUR/PBRER cycle. No fixed calendar deadline.
    Periodic,
    /// Reference data with no reporting deadline (drug labels, guidelines, etc.)
    NonPerishable,
}

impl Perishability {
    /// Standard expedited reporting: 15 calendar days (fatal/life-threatening).
    pub const EXPEDITED_15: Self = Self::Expedited { deadline_days: 15 };
    /// Standard prompt reporting: 90 calendar days (serious, non-fatal).
    pub const PROMPT_90: Self = Self::Prompt { deadline_days: 90 };

    /// Urgency rank for comparison (lower = more urgent).
    #[must_use]
    pub fn urgency_rank(&self) -> u8 {
        match self {
            Self::Expedited { .. } => 0,
            Self::Prompt { .. } => 1,
            Self::Periodic => 2,
            Self::NonPerishable => 3,
        }
    }

    /// Whether this perishability has a fixed calendar deadline.
    #[must_use]
    pub fn has_deadline(&self) -> bool {
        matches!(self, Self::Expedited { .. } | Self::Prompt { .. })
    }

    /// Get the deadline in days, if applicable.
    #[must_use]
    pub fn deadline_days(&self) -> Option<u16> {
        match self {
            Self::Expedited { deadline_days } | Self::Prompt { deadline_days } => {
                Some(*deadline_days)
            }
            _ => None,
        }
    }

    /// Attempt to upgrade perishability to a more urgent level.
    ///
    /// Returns the more urgent of `self` and `new`. Perishability can only
    /// upgrade (become more urgent), never downgrade — once you know something
    /// is expedited, it stays expedited.
    #[must_use]
    pub fn upgrade(&self, new: Self) -> Self {
        if new.urgency_rank() < self.urgency_rank() {
            new
        } else {
            *self
        }
    }

    /// Whether this is more urgent than another perishability level.
    #[must_use]
    pub fn is_more_urgent_than(&self, other: &Self) -> bool {
        self.urgency_rank() < other.urgency_rank()
    }
}

impl fmt::Display for Perishability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expedited { deadline_days } => {
                write!(f, "Expedited ({deadline_days}-day)")
            }
            Self::Prompt { deadline_days } => {
                write!(f, "Prompt ({deadline_days}-day)")
            }
            Self::Periodic => write!(f, "Periodic"),
            Self::NonPerishable => write!(f, "Non-perishable"),
        }
    }
}

impl PartialOrd for Perishability {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.urgency_rank().cmp(&other.urgency_rank()).reverse())
    }
}

impl Ord for Perishability {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse: lower rank = more urgent = greater in ordering
        self.urgency_rank().cmp(&other.urgency_rank()).reverse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urgency_ordering() {
        let expedited = Perishability::EXPEDITED_15;
        let prompt = Perishability::PROMPT_90;
        let periodic = Perishability::Periodic;
        let non_perish = Perishability::NonPerishable;

        assert!(expedited > prompt);
        assert!(prompt > periodic);
        assert!(periodic > non_perish);
    }

    #[test]
    fn test_upgrade_only_increases_urgency() {
        let periodic = Perishability::Periodic;
        let prompt = Perishability::PROMPT_90;
        let expedited = Perishability::EXPEDITED_15;

        // Upgrade from periodic to prompt
        let upgraded = periodic.upgrade(prompt);
        assert_eq!(upgraded, prompt);

        // Upgrade from prompt to expedited
        let upgraded = prompt.upgrade(expedited);
        assert_eq!(upgraded, expedited);

        // Cannot downgrade from expedited to periodic
        let not_downgraded = expedited.upgrade(periodic);
        assert_eq!(not_downgraded, expedited);
    }

    #[test]
    fn test_deadline_days() {
        assert_eq!(Perishability::EXPEDITED_15.deadline_days(), Some(15));
        assert_eq!(Perishability::PROMPT_90.deadline_days(), Some(90));
        assert_eq!(Perishability::Periodic.deadline_days(), None);
        assert_eq!(Perishability::NonPerishable.deadline_days(), None);
    }

    #[test]
    fn test_has_deadline() {
        assert!(Perishability::EXPEDITED_15.has_deadline());
        assert!(Perishability::PROMPT_90.has_deadline());
        assert!(!Perishability::Periodic.has_deadline());
        assert!(!Perishability::NonPerishable.has_deadline());
    }

    #[test]
    fn test_display() {
        assert_eq!(
            Perishability::EXPEDITED_15.to_string(),
            "Expedited (15-day)"
        );
        assert_eq!(Perishability::PROMPT_90.to_string(), "Prompt (90-day)");
        assert_eq!(Perishability::Periodic.to_string(), "Periodic");
    }
}
