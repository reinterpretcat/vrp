#[cfg(test)]
#[path = "../../../tests/unit/extensions/analyze/territory_test.rs"]
mod territory_test;

use serde::Serialize;
use vrp_core::models::Problem as CoreProblem;
use vrp_core::prelude::*;
use vrp_pragmatic::format::problem::{
    BalancePeriodMetric, Objective, Problem as ApiProblem, TerritoryDerivation, TerritoryProximity, derive_territory,
};

/// The settings the territory derivation depends on, besides the problem itself. Both change the
/// answer, so a dump is only comparable against another dump taken with the same pair — which is
/// why they are echoed back in [`TerritoryDerivationReport`] rather than left implicit.
#[derive(Clone, Debug)]
pub struct TerritorySettings {
    /// Proximity metric defining the territory.
    pub proximity: TerritoryProximity,
    /// Balance metric the seeds equalize on; `None` ⇒ plain compact k-medoids.
    pub balance: Option<BalancePeriodMetric>,
}

/// The derivation plus the settings it was taken under. `anchors` and `weights` sit at the top
/// level in exactly the wire shape the `territory` objective accepts, so a dump can be pasted back
/// into a problem after dropping the two setting keys.
#[derive(Debug, Serialize)]
pub struct TerritoryDerivationReport {
    /// The proximity metric the derivation ran with.
    pub proximity: TerritoryProximity,
    /// The balance metric the derivation ran with; omitted when there was none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<BalancePeriodMetric>,
    /// The derived anchors and weights, keyed by driver id (else vehicle id).
    #[serde(flatten)]
    pub derivation: TerritoryDerivation,
}

/// Reads the `territory` objective's settings from a problem, searching nested multi-objectives.
/// `None` when the problem configures no territory objective at all.
pub fn find_territory_settings(api_problem: &ApiProblem) -> Option<TerritorySettings> {
    fn search(objectives: &[Objective]) -> Option<TerritorySettings> {
        objectives.iter().find_map(|objective| match objective {
            Objective::Territory { proximity, balance, .. } => {
                Some(TerritorySettings { proximity: *proximity, balance: balance.clone() })
            }
            Objective::MultiObjective { objectives, .. } => search(objectives),
            _ => None,
        })
    }

    api_problem.objectives.as_deref().and_then(search)
}

/// Runs the solver's territory derivation for `core_problem` and renders it as stable, sorted JSON.
///
/// The maps are ordered, the output is pretty-printed one driver per line and ends in a newline, so
/// two dumps can be compared with a plain `diff` and a port of the derivation can be checked for
/// byte-identical output.
pub fn get_territory_derivation(core_problem: &CoreProblem, settings: &TerritorySettings) -> GenericResult<String> {
    let derivation = derive_territory(core_problem, settings.proximity, settings.balance.clone());
    let report =
        TerritoryDerivationReport { proximity: settings.proximity, balance: settings.balance.clone(), derivation };

    serde_json::to_string_pretty(&report)
        .map(|json| format!("{json}\n"))
        .map_err(|err| format!("cannot serialize territory derivation: '{err}'").into())
}
