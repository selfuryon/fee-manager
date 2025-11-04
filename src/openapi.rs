use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(title = "FeeManager API"),
    servers(
        (url = "{server_url}", variables(
            ("server_url" = (default = "http://localhost:3000", description = "Local server"))
        )),
    ),
    paths(
        crate::handlers::default_config::get_default_config,
        crate::handlers::default_config::create_or_update_default_config,
        crate::handlers::get_ready,
    ),
    tags(
        (name = "Health", description = "Service health"),
        (name = "Default config", description = "Set of endpoints to work with default config"),
    )
)]
pub struct ApiDoc;
