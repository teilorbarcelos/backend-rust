pub mod controller;
pub mod routes;

pub use routes::router;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(
    controller::trigger_pdf_post_handler,
    controller::trigger_pdf_get_handler,
))]
pub struct DebugApi;
