use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

use crate::routes::{
    auth::{RegisterDeviceBody, RegisterDeviceResponse, RegisterIdentityBody},
    board_shares::{BoardShareResponse, ShareBoardBody, SharedBoardResponse},
    boards::{BoardResponse, CreateBoardBody, PatchBoardBody, ReorderItem},
    devices::{DeviceResponse, RenameBody},
    health::HealthResponse,
    identity::{IdentityResponse, UpdateNameBody},
    invites::{CreateInviteBody, InviteResponse},
    link::{ConfirmLinkBody, InitLinkResponse, LinkInfoResponse, LinkStatusResponse},
    notes::{CreateNoteBody, NoteMetadata, PatchNoteBody},
    register::{RegisterBody, RegisterResponse},
    shares::{ShareBody, ShareEntry, SharedNoteEntry},
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "jot API",
        version = "1.0.0",
        description = "Universal encrypted note system — digital post-its. \
            Identity is a local UUID with a friendly name; no email or password required."
    ),
    paths(
        crate::routes::health::health,
        crate::routes::register::register,
        crate::routes::auth::register_identity,
        crate::routes::auth::register_device,
        crate::routes::link::init_link,
        crate::routes::link::get_link,
        crate::routes::link::confirm_link,
        crate::routes::link::link_status,
        crate::routes::identity::get_me,
        crate::routes::identity::update_me,
        crate::routes::identity::get_recent_contacts,
        crate::routes::identity::lookup_by_name,
        crate::routes::boards::list_boards,
        crate::routes::boards::create_board,
        crate::routes::boards::patch_board,
        crate::routes::boards::delete_board,
        crate::routes::boards::reorder_board,
        crate::routes::board_shares::get_boards_shared_with_me,
        crate::routes::board_shares::list_board_shares,
        crate::routes::board_shares::share_board,
        crate::routes::board_shares::revoke_board_share,
        crate::routes::notes::list_notes,
        crate::routes::notes::create_note,
        crate::routes::notes::get_note,
        crate::routes::notes::patch_note,
        crate::routes::notes::delete_note,
        crate::routes::notes::get_blob,
        crate::routes::notes::put_blob,
        crate::routes::shares::get_shared_with_me,
        crate::routes::shares::list_shares,
        crate::routes::shares::share_note,
        crate::routes::shares::delete_share,
        crate::routes::invites::list_invites,
        crate::routes::invites::create_invite,
        crate::routes::invites::revoke_invite,
        crate::routes::devices::list_devices,
        crate::routes::devices::delete_device,
        crate::routes::devices::rename_device,
        crate::routes::export::export_data,
    ),
    components(schemas(
        HealthResponse,
        RegisterBody,
        RegisterResponse,
        RegisterIdentityBody,
        RegisterDeviceBody,
        RegisterDeviceResponse,
        InitLinkResponse,
        LinkInfoResponse,
        LinkStatusResponse,
        ConfirmLinkBody,
        IdentityResponse,
        UpdateNameBody,
        BoardResponse,
        CreateBoardBody,
        PatchBoardBody,
        ReorderItem,
        BoardShareResponse,
        ShareBoardBody,
        SharedBoardResponse,
        NoteMetadata,
        CreateNoteBody,
        PatchNoteBody,
        ShareEntry,
        SharedNoteEntry,
        ShareBody,
        InviteResponse,
        CreateInviteBody,
        DeviceResponse,
        RenameBody,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Health check"),
        (name = "auth", description = "Registration and device authentication"),
        (name = "link", description = "Device linking via QR code flow"),
        (name = "identity", description = "Identity and friendly name management"),
        (name = "boards", description = "Board CRUD"),
        (name = "notes", description = "Note CRUD and blob storage"),
        (name = "shares", description = "Note and board sharing"),
        (name = "invites", description = "Invite token management"),
        (name = "devices", description = "Device management"),
        (name = "export", description = "Full data export"),
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}
