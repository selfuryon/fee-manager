use utoipa::OpenApi;

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
    paths(
        // Health
        crate::handlers::get_ready,
        crate::handlers::get_health,
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
            crate::schema::PaginatedResponse<crate::schema::ProposerListItem>,
            crate::schema::PaginatedResponse<crate::schema::DefaultConfigListItem>,
            crate::schema::PaginatedResponse<crate::schema::ProposerPatternListItem>,
            crate::schema::PaginatedResponse<crate::schema::MuxConfigListItem>,
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
            crate::schema::ExecutionConfigRequest,
            crate::schema::ExecutionConfigResponse,
            crate::schema::ProposerEntry,
            // Commit-Boost - Mux
            crate::schema::MuxConfigResponse,
            crate::schema::MuxConfigListItem,
            crate::schema::CreateMuxConfigRequest,
            crate::schema::UpdateMuxConfigRequest,
            crate::schema::MuxKeysRequest,
            crate::schema::MuxKeysResponse,
        )
    ),
    tags(
        (name = "Health", description = "Service health endpoints"),
        (name = "Vouch - Public", description = "Public Vouch endpoints for execution configuration"),
        (name = "Vouch - Proposers", description = "Admin endpoints for managing proposer configurations"),
        (name = "Vouch - Default Configs", description = "Admin endpoints for managing default configurations"),
        (name = "Vouch - Proposer Patterns", description = "Admin endpoints for managing proposer patterns"),
        (name = "Commit-Boost - Public", description = "Public Commit-Boost endpoints"),
        (name = "Commit-Boost - Mux", description = "Admin endpoints for managing mux configurations"),
    )
)]
pub struct ApiDoc;
