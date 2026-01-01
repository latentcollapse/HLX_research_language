"""
HLX Runtime Error Codes
Standardized error codes for the HLX runtime system.
"""

# Parse/Decode Errors
E_LC_PARSE = "E_LC_PARSE"
E_LC_DECODE = "E_LC_DECODE"
E_LC_ENCODE = "E_LC_ENCODE"
E_DEPTH_EXCEEDED = "E_DEPTH_EXCEEDED"
E_FLOAT_SPECIAL = "E_FLOAT_SPECIAL"
E_TRAILING_COMMA = "E_TRAILING_COMMA"

# Contract Errors
E_CONTRACT_STRUCTURE = "E_CONTRACT_STRUCTURE"
E_CONTRACT_UNKNOWN = "E_CONTRACT_UNKNOWN"
E_CONTRACT_FIELD_MISSING = "E_CONTRACT_FIELD_MISSING"
E_CONTRACT_FIELD_TYPE = "E_CONTRACT_FIELD_TYPE"
E_TYPE_MISMATCH = "E_TYPE_MISMATCH"

# Handle Errors
E_HANDLE_UNRESOLVED = "E_HANDLE_UNRESOLVED"
E_HANDLE_FORMAT = "E_HANDLE_FORMAT"
E_HANDLE_COLLISION = "E_HANDLE_COLLISION"
E_HANDLE_NOT_FOUND = "E_HANDLE_NOT_FOUND"

# Validation Errors
E_VALIDATION_FAIL = "E_VALIDATION_FAIL"
E_CONFORMANCE_VIOLATION = "E_CONFORMANCE_VIOLATION"
E_SCHEMA_MISMATCH = "E_SCHEMA_MISMATCH"

# Integrity Errors
E_ENV_PAYLOAD_HASH_MISMATCH = "E_ENV_PAYLOAD_HASH_MISMATCH"
E_MERKLE_VALIDATION_FAIL = "E_MERKLE_VALIDATION_FAIL"

# IO Errors
E_IO_ERROR = "E_IO_ERROR"
E_CAS_WRITE_FAIL = "E_CAS_WRITE_FAIL"
E_CAS_READ_FAIL = "E_CAS_READ_FAIL"

# Input Errors
E_INVALID_INPUT = "E_INVALID_INPUT"
E_MISSING_PARAMETER = "E_MISSING_PARAMETER"

# Determinism Errors
E_NONDETERMINISTIC = "E_NONDETERMINISTIC"
E_FIELD_ORDER = "E_FIELD_ORDER"
E_DUPLICATE_KEY = "E_DUPLICATE_KEY"
E_KEY_ORDER = "E_KEY_ORDER" # Alias for FIELD_ORDER in some contexts


class HLXError(Exception):
    """Base HLX runtime error."""
    def __init__(self, code: str, message: str):
        self.code = code
        self.message = message
        super().__init__(f"{code}: {message}")


class ParseError(HLXError):
    """LC parse/decode error."""
    def __init__(self, message: str):
        super().__init__(E_LC_PARSE, message)


class ContractError(HLXError):
    """Contract validation error."""
    def __init__(self, message: str):
        super().__init__(E_CONTRACT_STRUCTURE, message)


class HandleError(HLXError):
    """Handle resolution error."""
    def __init__(self, message: str, code: str = E_HANDLE_UNRESOLVED):
        super().__init__(code, message)


class HandleNotFoundError(HandleError):
    """Handle not found error."""
    def __init__(self, message: str):
        super().__init__(message, E_HANDLE_NOT_FOUND)


class IntegrityError(HLXError):
    """Hash/integrity validation error."""
    def __init__(self, message: str):
        super().__init__(E_ENV_PAYLOAD_HASH_MISMATCH, message)