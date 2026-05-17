//! C-ABI bindings for the Flutter mobile app.
//!
//! Each exported function follows the same contract:
//! - Inputs are passed either as raw byte pointers (`*const u8`, `len: usize`)
//!   or as null-terminated UTF-8 strings (`*const c_char`).
//! - Variable-size outputs are returned via an out parameter (`*mut JotBuffer`).
//!   Memory ownership is transferred to the caller, which MUST free it through
//!   `jot_buffer_free`.
//! - Fixed-size outputs (32-byte keys) are written into a caller-allocated
//!   `*mut u8` of the appropriate size.
//! - The function return value is `0` on success, non-zero on error.

use std::ffi::CStr;
use std::os::raw::c_char;

use jot_core::crypto::{
    decrypt, derive_bek, derive_dek, derive_wrap_key, encrypt, generate_dek,
    generate_static_keypair, static_diffie_hellman,
};
use x25519_dalek::{PublicKey as XPublicKey, StaticSecret as XStaticSecret};

#[repr(C)]
pub struct JotBuffer {
    pub data: *mut u8,
    pub len: usize,
}

impl JotBuffer {
    fn from_vec(mut v: Vec<u8>) -> Self {
        v.shrink_to_fit();
        let len = v.len();
        let ptr = v.as_mut_ptr();
        std::mem::forget(v);
        JotBuffer { data: ptr, len }
    }
    fn empty() -> Self {
        JotBuffer {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }
}

/// Free a buffer previously returned via an out parameter.
///
/// # Safety
/// `buf` must originate from a `JotBuffer::from_vec` allocation made by this
/// crate (no double-free, no foreign allocators).
#[no_mangle]
pub unsafe extern "C" fn jot_buffer_free(buf: JotBuffer) {
    if buf.data.is_null() || buf.len == 0 {
        return;
    }
    let _ = Vec::from_raw_parts(buf.data, buf.len, buf.len);
}

unsafe fn slice<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(ptr, len)
    }
}

unsafe fn key32(ptr: *const u8) -> Option<[u8; 32]> {
    if ptr.is_null() {
        return None;
    }
    let mut out = [0u8; 32];
    std::ptr::copy_nonoverlapping(ptr, out.as_mut_ptr(), 32);
    Some(out)
}

unsafe fn id16_from_uuid_cstr(ptr: *const c_char) -> Option<[u8; 16]> {
    if ptr.is_null() {
        return None;
    }
    let s = CStr::from_ptr(ptr).to_str().ok()?;
    let id = uuid::Uuid::parse_str(s).ok()?;
    Some(*id.as_bytes())
}

// ── crypto: symmetric ────────────────────────────────────────────────────────

/// Encrypt `plaintext` under a 32-byte AES-256-GCM key. Returns
/// `[12-byte nonce || ciphertext+tag]`.
///
/// # Safety
/// `key` must point to 32 readable bytes; `plaintext` must point to
/// `plaintext_len` readable bytes (or be null when `plaintext_len == 0`);
/// `out` must point to a writable `JotBuffer`.
#[no_mangle]
pub unsafe extern "C" fn jot_encrypt(
    key: *const u8,
    plaintext: *const u8,
    plaintext_len: usize,
    out: *mut JotBuffer,
) -> i32 {
    let Some(k) = key32(key) else { return 1 };
    let pt = slice(plaintext, plaintext_len);
    match encrypt(&k, pt) {
        Ok(blob) => {
            *out = JotBuffer::from_vec(blob);
            0
        }
        Err(_) => {
            *out = JotBuffer::empty();
            2
        }
    }
}

/// Decrypt a `[nonce(12) || ciphertext+tag]` blob.
///
/// # Safety
/// Same constraints as `jot_encrypt` for the input pointers.
#[no_mangle]
pub unsafe extern "C" fn jot_decrypt(
    key: *const u8,
    blob: *const u8,
    blob_len: usize,
    out: *mut JotBuffer,
) -> i32 {
    let Some(k) = key32(key) else { return 1 };
    let b = slice(blob, blob_len);
    match decrypt(&k, b) {
        Ok(pt) => {
            *out = JotBuffer::from_vec(pt);
            0
        }
        Err(_) => {
            *out = JotBuffer::empty();
            2
        }
    }
}

/// Generate a fresh random 32-byte DEK.
///
/// # Safety
/// `out` must point to 32 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn jot_generate_dek(out: *mut u8) -> i32 {
    if out.is_null() {
        return 1;
    }
    let dek = generate_dek();
    std::ptr::copy_nonoverlapping(dek.as_ptr(), out, 32);
    0
}

// ── crypto: key derivation ───────────────────────────────────────────────────

/// Derive a per-board encryption key from an identity private key.
///
/// # Safety
/// `privkey` must point to 32 readable bytes. `board_id` must be a
/// null-terminated UUID string. `out` must point to 32 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn jot_derive_bek(
    privkey: *const u8,
    board_id: *const c_char,
    out: *mut u8,
) -> i32 {
    let Some(pk) = key32(privkey) else { return 1 };
    let Some(id) = id16_from_uuid_cstr(board_id) else {
        return 2;
    };
    match derive_bek(&pk, &id) {
        Ok(bek) => {
            std::ptr::copy_nonoverlapping(bek.as_ptr(), out, 32);
            0
        }
        Err(_) => 3,
    }
}

/// Derive a per-note DEK from a BEK and note UUID.
///
/// # Safety
/// Same constraints as `jot_derive_bek`.
#[no_mangle]
pub unsafe extern "C" fn jot_derive_dek(
    bek: *const u8,
    note_id: *const c_char,
    out: *mut u8,
) -> i32 {
    let Some(b) = key32(bek) else { return 1 };
    let Some(id) = id16_from_uuid_cstr(note_id) else {
        return 2;
    };
    match derive_dek(&b, &id) {
        Ok(dek) => {
            std::ptr::copy_nonoverlapping(dek.as_ptr(), out, 32);
            0
        }
        Err(_) => 3,
    }
}

/// Derive a DEK-wrapping key from an X25519 shared secret.
///
/// # Safety
/// `shared_secret` and `out` must each point to 32 readable / writable bytes.
#[no_mangle]
pub unsafe extern "C" fn jot_derive_wrap_key(shared_secret: *const u8, out: *mut u8) -> i32 {
    let Some(s) = key32(shared_secret) else {
        return 1;
    };
    match derive_wrap_key(&s) {
        Ok(k) => {
            std::ptr::copy_nonoverlapping(k.as_ptr(), out, 32);
            0
        }
        Err(_) => 2,
    }
}

// ── crypto: X25519 ───────────────────────────────────────────────────────────

/// Generate a fresh X25519 static keypair (identity key).
///
/// # Safety
/// `privkey_out` and `pubkey_out` must each point to 32 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn jot_generate_static_keypair(
    privkey_out: *mut u8,
    pubkey_out: *mut u8,
) -> i32 {
    let (secret, public) = generate_static_keypair();
    let priv_bytes = secret.to_bytes();
    std::ptr::copy_nonoverlapping(priv_bytes.as_ptr(), privkey_out, 32);
    std::ptr::copy_nonoverlapping(public.as_bytes().as_ptr(), pubkey_out, 32);
    0
}

/// Compute the X25519 public key for a given private key.
///
/// # Safety
/// `privkey` must point to 32 readable bytes; `pubkey_out` to 32 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn jot_pubkey_from_privkey(privkey: *const u8, pubkey_out: *mut u8) -> i32 {
    let Some(pk) = key32(privkey) else { return 1 };
    let secret = XStaticSecret::from(pk);
    let public = XPublicKey::from(&secret);
    std::ptr::copy_nonoverlapping(public.as_bytes().as_ptr(), pubkey_out, 32);
    0
}

/// X25519 ECDH using a reusable static private key.
///
/// # Safety
/// `privkey` and `peer_pubkey` must each point to 32 readable bytes; `out` to
/// 32 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn jot_static_diffie_hellman(
    privkey: *const u8,
    peer_pubkey: *const u8,
    out: *mut u8,
) -> i32 {
    let Some(priv_k) = key32(privkey) else {
        return 1;
    };
    let Some(pub_k) = key32(peer_pubkey) else {
        return 2;
    };
    let secret = XStaticSecret::from(priv_k);
    let public = XPublicKey::from(pub_k);
    let shared = static_diffie_hellman(&secret, &public);
    std::ptr::copy_nonoverlapping(shared.as_ptr(), out, 32);
    0
}

// ── blocks: markdown link extraction ─────────────────────────────────────────

#[derive(serde::Serialize)]
struct LinkOut {
    target_kind: &'static str,
    target_id: String,
    link_kind: &'static str,
}

/// Extract `[[page]]`, `((block))`, and `#tag` references from a markdown
/// string and return them as JSON: `[{target_kind, target_id, link_kind}]`.
///
/// Page references are returned with their raw title text; the caller is
/// responsible for resolving titles to note IDs on the Dart side.
///
/// # Safety
/// `markdown` must be a null-terminated UTF-8 string. `out` must point to a
/// writable `JotBuffer`.
#[no_mangle]
pub unsafe extern "C" fn jot_extract_links(markdown: *const c_char, out: *mut JotBuffer) -> i32 {
    if markdown.is_null() {
        *out = JotBuffer::empty();
        return 1;
    }
    let s = match CStr::from_ptr(markdown).to_str() {
        Ok(s) => s,
        Err(_) => {
            *out = JotBuffer::empty();
            return 2;
        }
    };
    let links = jot_core::blocks::extract_links(s, &std::collections::HashMap::new());
    let serialised: Vec<LinkOut> = links
        .into_iter()
        .map(|l| LinkOut {
            target_kind: l.target_kind.as_str(),
            target_id: l.target_id,
            link_kind: l.link_kind.as_str(),
        })
        .collect();
    let json = serde_json::to_vec(&serialised).unwrap_or_default();
    *out = JotBuffer::from_vec(json);
    0
}

/// Library version (semver) as a UTF-8 byte buffer.
///
/// # Safety
/// `out` must point to a writable `JotBuffer`.
#[no_mangle]
pub unsafe extern "C" fn jot_version(out: *mut JotBuffer) -> i32 {
    *out = JotBuffer::from_vec(env!("CARGO_PKG_VERSION").as_bytes().to_vec());
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let key = [9u8; 32];
        let plaintext = b"sketches on the train";
        let mut blob = JotBuffer::empty();
        unsafe {
            assert_eq!(
                jot_encrypt(key.as_ptr(), plaintext.as_ptr(), plaintext.len(), &mut blob),
                0
            );
            let mut out = JotBuffer::empty();
            assert_eq!(jot_decrypt(key.as_ptr(), blob.data, blob.len, &mut out), 0);
            let recovered = std::slice::from_raw_parts(out.data, out.len).to_vec();
            assert_eq!(recovered, plaintext);
            jot_buffer_free(blob);
            jot_buffer_free(out);
        }
    }

    #[test]
    fn bek_then_dek_chain() {
        let priv_key = [3u8; 32];
        let board = std::ffi::CString::new("11111111-1111-1111-1111-111111111111").unwrap();
        let note = std::ffi::CString::new("22222222-2222-2222-2222-222222222222").unwrap();
        let mut bek = [0u8; 32];
        let mut dek = [0u8; 32];
        unsafe {
            assert_eq!(
                jot_derive_bek(priv_key.as_ptr(), board.as_ptr(), bek.as_mut_ptr()),
                0
            );
            assert_eq!(
                jot_derive_dek(bek.as_ptr(), note.as_ptr(), dek.as_mut_ptr()),
                0
            );
        }
        assert_ne!(bek, [0u8; 32]);
        assert_ne!(dek, bek);
    }

    #[test]
    fn static_dh_symmetric() {
        let mut a_priv = [0u8; 32];
        let mut a_pub = [0u8; 32];
        let mut b_priv = [0u8; 32];
        let mut b_pub = [0u8; 32];
        let mut ab = [0u8; 32];
        let mut ba = [0u8; 32];
        unsafe {
            jot_generate_static_keypair(a_priv.as_mut_ptr(), a_pub.as_mut_ptr());
            jot_generate_static_keypair(b_priv.as_mut_ptr(), b_pub.as_mut_ptr());
            jot_static_diffie_hellman(a_priv.as_ptr(), b_pub.as_ptr(), ab.as_mut_ptr());
            jot_static_diffie_hellman(b_priv.as_ptr(), a_pub.as_ptr(), ba.as_mut_ptr());
        }
        assert_eq!(ab, ba);
    }

    #[test]
    fn extract_links_emits_tags() {
        let md = std::ffi::CString::new("hello #world").unwrap();
        let mut out = JotBuffer::empty();
        unsafe {
            jot_extract_links(md.as_ptr(), &mut out);
            let bytes = std::slice::from_raw_parts(out.data, out.len);
            let s = std::str::from_utf8(bytes).unwrap();
            assert!(s.contains("\"tag\""));
            assert!(s.contains("\"world\""));
            jot_buffer_free(out);
        }
    }
}
