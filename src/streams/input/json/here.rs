
mod parser {
    extern crate serde_json;
    use serde::Deserialize;
    use serde_json::Result;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    enum RelationType {
        Tour,
        Flexible,
        Sequence,
    }
}