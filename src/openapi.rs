use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::{ContentBuilder, Ref, RefOr, ResponseBuilder};
use utoipa::{Modify, OpenApi};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Fee Manager API",
        description = "REST API for managing validator configurations for Vouch and Commit-Boost",
        version = "1.0.0"
    ),
    servers(
        (url = "{server_url}", variables(
            ("server_url" = (default = "http://localhost:3000", description = "API Server URL"))
        )),
    ),
    modifiers(&SecurityAddon, &ErrorResponsesAddon),
    paths(
        // Health
        crate::handlers::get_ready,
        crate::handlers::get_health,
        // Auth
        crate::auth::handlers::list_tokens,
        crate::auth::handlers::create_token,
        crate::auth::handlers::delete_token,
        // Vouch - Public
        crate::handlers::vouch::execution_config::get_execution_config,
        // Vouch - Proposers
        crate::handlers::vouch::proposers::list_proposers,
        crate::handlers::vouch::proposers::get_proposer,
        crate::handlers::vouch::proposers::create_or_update_proposer,
        crate::handlers::vouch::proposers::delete_proposer,
        // Vouch - Default Configs
        crate::handlers::vouch::default_configs::list_default_configs,
        crate::handlers::vouch::default_configs::get_default_config,
        crate::handlers::vouch::default_configs::create_default_config,
        crate::handlers::vouch::default_configs::update_default_config,
        crate::handlers::vouch::default_configs::delete_default_config,
        // Vouch - Proposer Patterns
        crate::handlers::vouch::proposer_patterns::list_proposer_patterns,
        crate::handlers::vouch::proposer_patterns::get_proposer_pattern,
        crate::handlers::vouch::proposer_patterns::create_proposer_pattern,
        crate::handlers::vouch::proposer_patterns::update_proposer_pattern,
        crate::handlers::vouch::proposer_patterns::delete_proposer_pattern,
        // Commit-Boost - Public
        crate::handlers::commit_boost::mux::get_mux_keys_public,
        // Commit-Boost - Mux Admin
        crate::handlers::commit_boost::mux::list_mux_configs,
        crate::handlers::commit_boost::mux::get_mux_config,
        crate::handlers::commit_boost::mux::create_mux_config,
        crate::handlers::commit_boost::mux::update_mux_config,
        crate::handlers::commit_boost::mux::delete_mux_config,
        crate::handlers::commit_boost::mux::add_mux_keys,
        crate::handlers::commit_boost::mux::remove_mux_keys,
    ),
    components(
        schemas(
            crate::handlers::HealthResponse,
            crate::errors::ErrorResponse,
            crate::errors::ErrorDetail,
            // Common
            crate::schema::RelayConfig,
            crate::schema::ProposerRelayConfig,
            crate::pagination::ListResponse<crate::schema::ProposerListItem>,
            crate::pagination::ListResponse<crate::schema::DefaultConfigListItem>,
            crate::pagination::ListResponse<crate::schema::ProposerPatternListItem>,
            crate::pagination::ListResponse<crate::schema::MuxConfigListItem>,
            // Vouch - Proposers
            crate::schema::ProposerResponse,
            crate::schema::ProposerListItem,
            crate::schema::CreateOrUpdateProposerRequest,
            // Vouch - Default Configs
            crate::schema::DefaultConfigResponse,
            crate::schema::DefaultConfigListItem,
            crate::schema::CreateDefaultConfigRequest,
            crate::schema::UpdateDefaultConfigRequest,
            // Vouch - Proposer Patterns
            crate::schema::ProposerPatternResponse,
            crate::schema::ProposerPatternListItem,
            crate::schema::CreateProposerPatternRequest,
            crate::schema::UpdateProposerPatternRequest,
            // Vouch - Execution Config
            crate::schema::ExecutionConfigResponse,
            crate::schema::ProposerEntry,
            // Commit-Boost - Mux
            crate::schema::MuxConfigResponse,
            crate::schema::MuxConfigListItem,
            crate::schema::CreateMuxConfigRequest,
            crate::schema::UpdateMuxConfigRequest,
            crate::schema::MuxKeysRequest,
            crate::schema::MuxKeysResponse,
            // Auth
            crate::auth::TokenInfo,
            crate::auth::handlers::CreateTokenRequest,
            crate::auth::handlers::CreateTokenResponse,
        )
    ),
    tags(
        (name = "Health", description = "Service health endpoints"),
        (name = "Auth", description = "API token management"),
        (name = "Vouch - Public", description = "Public Vouch endpoints for execution configuration"),
        (name = "Vouch - Proposers", description = "Admin endpoints for managing proposer configurations"),
        (name = "Vouch - Default Configs", description = "Admin endpoints for managing default configurations"),
        (name = "Vouch - Proposer Patterns", description = "Admin endpoints for managing proposer patterns"),
        (name = "Commit-Boost - Public", description = "Public Commit-Boost endpoints"),
        (name = "Commit-Boost - Mux", description = "Admin endpoints for managing mux configurations"),
    )
)]
pub struct ApiDoc;

/// Security scheme for Bearer token authentication
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("token")
                        .description(Some("API token for authentication"))
                        .build(),
                ),
            );
        }
    }
}

/// Documents the shared error responses on every operation, so the ~25
/// handler annotations do not each have to repeat them. A response an
/// operation already declares keeps its description and gains the
/// `ErrorResponse` body if it had none.
struct ErrorResponsesAddon;

impl Modify for ErrorResponsesAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let body = || {
            ContentBuilder::new()
                .schema(Some(Ref::from_schema_name("ErrorResponse")))
                .build()
        };
        for (path, item) in openapi.paths.paths.iter_mut() {
            let admin = path.starts_with("/api/admin/");
            let has_path_param = path.contains('{');
            // Probes take no input and do not fail
            if path == "/health" || path == "/ready" {
                continue;
            }
            let operations = [
                ("get", item.get.as_mut()),
                ("put", item.put.as_mut()),
                ("post", item.post.as_mut()),
                ("delete", item.delete.as_mut()),
                ("patch", item.patch.as_mut()),
            ];
            for (method, operation) in operations {
                let Some(operation) = operation else { continue };
                let mut statuses = vec![("400", "Invalid input"), ("500", "Internal error")];
                if admin {
                    statuses.push(("401", "Missing or invalid token"));
                }
                // PUT on a proposer creates it when missing
                let upsert = method == "put" && path.starts_with("/api/admin/vouch/proposers/");
                if has_path_param && !upsert {
                    statuses.push(("404", "Not found"));
                }
                // Named resources; token names are not unique
                if method == "post" && !has_path_param && !path.starts_with("/api/admin/tokens") {
                    statuses.push(("409", "Already exists"));
                }
                if path.starts_with("/vouch/") && method == "post" {
                    statuses.push(("413", "Request body too large"));
                }
                for (status, description) in statuses {
                    let response = operation
                        .responses
                        .responses
                        .entry(status.to_string())
                        .or_insert_with(|| {
                            RefOr::T(ResponseBuilder::new().description(description).build())
                        });
                    if let RefOr::T(response) = response
                        && response.content.is_empty()
                    {
                        response
                            .content
                            .insert("application/json".to_string(), RefOr::T(body()));
                    }
                }
            }
        }
    }
}
