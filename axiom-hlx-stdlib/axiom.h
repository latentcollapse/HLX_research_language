/**
 * axiom.h — Axiom verification engine, C API
 *
 * Embed Axiom in any language that can call C:
 *   Python (ctypes / cffi), Go (cgo), Node.js (ffi-napi),
 *   Ruby, Swift, Zig, and more.
 *
 * Usage
 * -----
 *   #include "axiom.h"
 *
 *   axiom_engine_t *eng = axiom_engine_open("security.axm");
 *
 *   const char *keys[] = { "path" };
 *   const char *vals[] = { "/tmp/out.txt" };
 *   int rc = axiom_verify(eng, "WriteFile", keys, vals, 1);
 *
 *   if      (rc == 1) puts("allowed");
 *   else if (rc == 0) printf("denied: %s\n", axiom_denied_reason(eng));
 *   else              printf("error: %s\n",  axiom_errmsg(eng));
 *
 *   axiom_engine_close(eng);
 *
 * Build
 * -----
 *   cargo build --release
 *   # shared lib:  target/release/libaxiom_lang.so  (Linux)
 *                  target/release/libaxiom_lang.dylib (macOS)
 *   # static lib:  target/release/libaxiom_lang.a
 */

#ifndef AXIOM_H
#define AXIOM_H

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque engine handle. */
typedef struct axiom_engine axiom_engine_t;

/* ── Open / Close ─────────────────────────────────────────────────────────── */

/**
 * Open an Axiom engine from a .axm policy file.
 *
 * Returns a handle on success, or a non-NULL handle whose axiom_errmsg()
 * is set on failure.  Never returns NULL (safe to pass directly to other
 * axiom_* calls; they will return -1 / empty string as appropriate).
 *
 * The caller must eventually call axiom_engine_close().
 */
axiom_engine_t *axiom_engine_open(const char *path);

/**
 * Open an Axiom engine from an in-memory policy source string.
 *
 * Same semantics as axiom_engine_open().
 */
axiom_engine_t *axiom_engine_open_source(const char *source);

/**
 * Close and free an engine handle.  Safe to call with NULL.
 */
void axiom_engine_close(axiom_engine_t *engine);

/* ── Verification ─────────────────────────────────────────────────────────── */

/**
 * Verify an intent against the loaded policy.
 *
 *   engine  — handle from axiom_engine_open / axiom_engine_open_source
 *   intent  — intent name, e.g. "WriteFile"
 *   keys    — array of n NUL-terminated key strings   (NULL if n == 0)
 *   values  — array of n NUL-terminated value strings (NULL if n == 0)
 *   n       — number of key-value field pairs
 *
 * Returns
 *    1  allowed
 *    0  denied  — call axiom_denied_reason() for policy guidance
 *   -1  error   — call axiom_errmsg() for the error message
 */
int axiom_verify(axiom_engine_t *engine,
                 const char     *intent,
                 const char    **keys,
                 const char    **values,
                 int             n);

/* ── Diagnostics ──────────────────────────────────────────────────────────── */

/**
 * Return the last error message, or "" if no error.
 * The pointer is valid until the next axiom_* call on this handle.
 */
const char *axiom_errmsg(axiom_engine_t *engine);

/**
 * Return the denial guidance from the last axiom_verify() call, or "".
 * The pointer is valid until the next axiom_* call on this handle.
 */
const char *axiom_denied_reason(axiom_engine_t *engine);

/* ── Metadata ─────────────────────────────────────────────────────────────── */

/**
 * Return the Axiom library version string (e.g. "0.1.0").
 */
const char *axiom_version(void);

#ifdef __cplusplus
}
#endif
#endif /* AXIOM_H */
