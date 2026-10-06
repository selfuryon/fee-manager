use fee_manager::openapi::ApiDoc;
use utoipa::OpenApi;

const SPEC_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");

/// The committed openapi.json is the published API contract; it must match
/// what the handlers actually expose. Regenerate with:
/// `UPDATE_OPENAPI=1 cargo test --test openapi_test`
#[test]
fn openapi_json_is_up_to_date() {
    let generated = ApiDoc::openapi()
        .to_pretty_json()
        .expect("OpenAPI spec serializes")
        + "\n";

    if std::env::var_os("UPDATE_OPENAPI").is_some() {
        std::fs::write(SPEC_PATH, &generated).expect("write openapi.json");
        return;
    }

    let committed = std::fs::read_to_string(SPEC_PATH).unwrap_or_default();
    assert!(
        committed == generated,
        "openapi.json is out of date; run `UPDATE_OPENAPI=1 cargo test --test openapi_test`"
    );
}
