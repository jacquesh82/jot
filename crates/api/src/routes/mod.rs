pub mod auth;
pub mod blocks;
pub mod board_keys;
pub mod board_shares;
pub mod boards;
pub mod devices;
pub mod export;
pub mod health;
pub mod identity;
pub mod invites;
pub mod link;
pub mod notes;
pub mod register;
pub mod shares;
pub mod spa;
pub mod tags;
pub mod ws;

use crate::openapi::ApiDoc;
use crate::state::AppState;
use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn build(state: AppState) -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/health", get(health::health))
        .route("/register", post(register::register))
        .route(
            "/invites",
            get(invites::list_invites).post(invites::create_invite),
        )
        .route("/invites/:token", delete(invites::revoke_invite))
        .route("/auth/register", post(auth::register_identity))
        .route("/auth/device", post(auth::register_device))
        .route("/link/init", post(link::init_link))
        .route("/link/:token", get(link::get_link))
        .route("/link/confirm", post(link::confirm_link))
        .route("/link/status/:token", get(link::link_status))
        .route(
            "/identity/me",
            get(identity::get_me).patch(identity::update_me).delete(identity::delete_identity),
        )
        .route("/identity/me/pubkey", put(identity::set_pubkey))
        .route("/identity/me/privkey", get(identity::get_privkey))
        .route("/identity/contacts", get(identity::get_recent_contacts))
        .route("/identity/lookup/:name", get(identity::lookup_by_name))
        .route("/notes", get(notes::list_notes).post(notes::create_note))
        .route("/notes/shared", get(shares::get_shared_with_me))
        .route("/notes/legacy-text", get(notes::list_legacy_text_notes))
        .route(
            "/notes/:id",
            get(notes::get_note)
                .delete(notes::delete_note)
                .patch(notes::patch_note),
        )
        .route(
            "/notes/:id/schema-version",
            patch(notes::patch_schema_version),
        )
        .route("/notes/:id/title", patch(notes::patch_note_title))
        .route("/notes/:id/blob", get(notes::get_blob).put(notes::put_blob))
        .route(
            "/notes/:note_id/blocks",
            get(blocks::list_blocks).post(blocks::create_block),
        )
        .route(
            "/blocks/:id",
            get(blocks::get_block)
                .patch(blocks::patch_block)
                .delete(blocks::delete_block),
        )
        .route("/blocks/:id/move", post(blocks::move_block))
        .route("/blocks/:id/indent", post(blocks::indent_block))
        .route("/blocks/:id/outdent", post(blocks::outdent_block))
        .route("/blocks/:id/backlinks", get(blocks::block_backlinks))
        .route("/blocks/:id/links", put(blocks::put_block_links))
        .route("/notes/:id/backlinks", get(blocks::note_backlinks))
        .route("/tags", get(tags::list_tags))
        .route("/tags/:name", put(tags::put_tag))
        .route("/tags/:name/blocks", get(tags::blocks_with_tag))
        .route(
            "/notes/:id/shares",
            get(shares::list_shares).post(shares::share_note),
        )
        .route(
            "/notes/:id/shares/:identity_id",
            delete(shares::delete_share),
        )
        .route("/notes/:id/dek", get(shares::get_dek).put(shares::put_dek))
        .route("/notes/:id/deks/:identity_id", put(shares::put_dek_for_identity))
        .route(
            "/boards",
            get(boards::list_boards).post(boards::create_board),
        )
        .route(
            "/boards/shared",
            get(board_shares::get_boards_shared_with_me),
        )
        .route(
            "/boards/:id",
            patch(boards::patch_board).delete(boards::delete_board),
        )
        .route("/boards/:id/reorder", patch(boards::reorder_board))
        .route(
            "/boards/:id/shares",
            get(board_shares::list_board_shares).post(board_shares::share_board),
        )
        .route(
            "/boards/:id/shares/:identity_id",
            delete(board_shares::revoke_board_share),
        )
        .route("/boards/:id/key", get(board_keys::get_board_key))
        .route(
            "/boards/:id/keys/:identity_id",
            put(board_keys::put_board_key).delete(board_keys::delete_board_key),
        )
        .route("/export", get(export::export_data))
        .route("/devices", get(devices::list_devices))
        .route("/devices/:id", delete(devices::delete_device))
        .route("/devices/:id/rename", post(devices::rename_device))
        .route("/ws", get(ws::ws_handler))
        .fallback(spa::spa_handler)
        .with_state(state)
}
