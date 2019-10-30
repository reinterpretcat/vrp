extern crate serde_json;

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum RelationType {
    Tour,
    Flexible,
    Sequence,
}
