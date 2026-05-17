// Stable C ABI exported by crates/mobile-ffi.
// Keep this file in sync with `crates/mobile-ffi/src/lib.rs`.
// `ffigen` consumes it to generate `lib/services/ffi/bindings_generated.dart`.

#ifndef JOT_MOBILE_FFI_H
#define JOT_MOBILE_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct JotBuffer {
  uint8_t *data;
  size_t len;
} JotBuffer;

void jot_buffer_free(JotBuffer buf);

int32_t jot_encrypt(const uint8_t *key,
                    const uint8_t *plaintext,
                    size_t plaintext_len,
                    JotBuffer *out);

int32_t jot_decrypt(const uint8_t *key,
                    const uint8_t *blob,
                    size_t blob_len,
                    JotBuffer *out);

int32_t jot_generate_dek(uint8_t *out);

int32_t jot_derive_bek(const uint8_t *privkey,
                       const char *board_id,
                       uint8_t *out);

int32_t jot_derive_dek(const uint8_t *bek,
                       const char *note_id,
                       uint8_t *out);

int32_t jot_derive_wrap_key(const uint8_t *shared_secret, uint8_t *out);

int32_t jot_generate_static_keypair(uint8_t *privkey_out, uint8_t *pubkey_out);

int32_t jot_pubkey_from_privkey(const uint8_t *privkey, uint8_t *pubkey_out);

int32_t jot_static_diffie_hellman(const uint8_t *privkey,
                                  const uint8_t *peer_pubkey,
                                  uint8_t *out);

int32_t jot_extract_links(const char *markdown, JotBuffer *out);

int32_t jot_version(JotBuffer *out);

#ifdef __cplusplus
}
#endif

#endif
