//! SPIR-V validation helpers
//!
//! Provides basic SPIR-V structure validation before passing to Vulkan.
//! This catches obvious errors early with better error messages than
//! Vulkan's generic validation errors.

/// Validate SPIR-V binary structure.
///
/// Performs basic sanity checks on the SPIR-V header:
/// - Magic number (0x07230203)
/// - Version compatibility (1.0 - 1.6)
/// - Non-zero bound
///
/// # Arguments
///
/// * `bytes` - Raw SPIR-V binary
///
/// # Returns
///
/// * `Ok(())` if basic structure is valid
/// * `Err(String)` with description if invalid
///
/// # Note
///
/// This is NOT a full SPIR-V validator. Use spirv-val for complete validation.
/// This function catches the most common issues quickly.
pub fn validate_spirv(bytes: &[u8]) -> Result<(), String> {
    // Check minimum size (header is 5 words = 20 bytes)
    if bytes.len() < 20 {
        return Err(format!(
            "SPIR-V too small: {} bytes (minimum 20)",
            bytes.len()
        ));
    }

    // Check 4-byte alignment
    if bytes.len() % 4 != 0 {
        return Err(format!(
            "SPIR-V size ({}) not 4-byte aligned",
            bytes.len()
        ));
    }

    // Check magic number (little-endian: 0x07230203)
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if magic != 0x07230203 {
        return Err(format!(
            "Invalid SPIR-V magic: 0x{:08x} (expected 0x07230203)",
            magic
        ));
    }

    // Check version (word 1)
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let major = (version >> 16) & 0xFF;
    let minor = (version >> 8) & 0xFF;

    // SPIR-V versions 1.0 through 1.6 are valid
    if major != 1 || minor > 6 {
        return Err(format!(
            "Unsupported SPIR-V version: {}.{} (supported: 1.0-1.6)",
            major, minor
        ));
    }

    // Word 2 is generator magic (skip validation)

    // Check bound (word 3) - must be > 0
    let bound = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    if bound == 0 {
        return Err("SPIR-V bound is 0 (invalid)".to_string());
    }

    // Word 4 is reserved (must be 0 per spec, but some tools violate this)

    log::trace!(
        "SPIR-V validated: version {}.{}, bound {}, size {}",
        major, minor, bound, bytes.len()
    );

    Ok(())
}

/// Check if bytes look like SPIR-V (quick magic check).
///
/// Useful for fast rejection of obviously wrong data.
#[inline]
pub fn is_spirv(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0..4] == [0x03, 0x02, 0x23, 0x07]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_spirv() -> Vec<u8> {
        // Minimal valid SPIR-V header (5 words = 20 bytes)
        // SPIR-V version word format (little-endian u32):
        // bits 0-7: 0
        // bits 8-15: minor version
        // bits 16-23: major version
        // bits 24-31: 0
        // So version 1.0 = 0x00010000, stored as [0x00, 0x00, 0x01, 0x00] in LE
        vec![
            0x03, 0x02, 0x23, 0x07,  // Magic (0x07230203 in LE)
            0x00, 0x00, 0x01, 0x00,  // Version 1.0 (0x00010000 in LE)
            0x00, 0x00, 0x00, 0x00,  // Generator magic
            0x01, 0x00, 0x00, 0x00,  // Bound = 1
            0x00, 0x00, 0x00, 0x00,  // Reserved
        ]
    }

    #[test]
    fn test_valid_spirv_header() {
        let spirv = make_valid_spirv();
        let result = validate_spirv(&spirv);
        assert!(result.is_ok(), "Expected valid SPIR-V, got: {:?}", result);
    }

    #[test]
    fn test_invalid_magic() {
        let mut spirv = make_valid_spirv();
        spirv[0] = 0x00; // Corrupt magic
        assert!(validate_spirv(&spirv).is_err());
    }

    #[test]
    fn test_too_small() {
        let spirv = vec![0x03, 0x02, 0x23, 0x07]; // Only magic, no header
        let result = validate_spirv(&spirv);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too small"));
    }

    #[test]
    fn test_not_aligned() {
        // 21 bytes - passes size check but fails alignment
        let spirv = vec![
            0x03, 0x02, 0x23, 0x07,  // Magic
            0x00, 0x00, 0x01, 0x00,  // Version
            0x00, 0x00, 0x00, 0x00,  // Generator
            0x01, 0x00, 0x00, 0x00,  // Bound
            0x00, 0x00, 0x00, 0x00,  // Reserved
            0x00,                     // Extra byte - breaks alignment
        ];
        let result = validate_spirv(&spirv);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("aligned"), "Expected alignment error, got: {}", err);
    }

    #[test]
    fn test_zero_bound() {
        let mut spirv = make_valid_spirv();
        // Bound is at word 3 (bytes 12-15), set to 0
        spirv[12] = 0x00;
        spirv[13] = 0x00;
        spirv[14] = 0x00;
        spirv[15] = 0x00;
        let result = validate_spirv(&spirv);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("bound"), "Expected bound error, got: {}", err);
    }

    #[test]
    fn test_is_spirv_quick_check() {
        assert!(is_spirv(&[0x03, 0x02, 0x23, 0x07]));
        assert!(!is_spirv(&[0x00, 0x00, 0x00, 0x00]));
        assert!(!is_spirv(&[0x03, 0x02, 0x23])); // Too short
    }

    #[test]
    fn test_version_range() {
        let mut spirv = make_valid_spirv();

        // Version 1.6 should be valid
        // Version word at bytes 4-7, minor at byte 5 (bits 8-15)
        spirv[5] = 0x06; // minor = 6, keeps major = 1
        let result = validate_spirv(&spirv);
        assert!(result.is_ok(), "Version 1.6 should be valid, got: {:?}", result);

        // Version 1.7 should be invalid (future version beyond 1.6)
        spirv[5] = 0x07;
        assert!(validate_spirv(&spirv).is_err());

        // Version 2.0 should be invalid
        spirv[5] = 0x00; // minor = 0
        spirv[6] = 0x02; // major = 2
        assert!(validate_spirv(&spirv).is_err());
    }
}
