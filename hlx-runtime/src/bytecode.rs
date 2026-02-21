const BYTECODE_MAGIC: &[u8; 4] = b"LC-B";
const BYTECODE_VERSION: u16 = 1;
const HEADER_SIZE: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytecodeError {
    InvalidMagic,
    UnsupportedVersion,
    IntegrityViolation,
    TruncatedData,
    DeserializationFailed(String),
}

impl std::fmt::Display for BytecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BytecodeError::InvalidMagic => write!(f, "Invalid bytecode magic number"),
            BytecodeError::UnsupportedVersion => write!(f, "Unsupported bytecode version"),
            BytecodeError::IntegrityViolation => write!(f, "Bytecode integrity check failed"),
            BytecodeError::TruncatedData => write!(f, "Bytecode data is truncated"),
            BytecodeError::DeserializationFailed(msg) => {
                write!(f, "Deserialization failed: {}", msg)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Nop = 0,
    Const = 1,
    Move = 2,
    Add = 10,
    Sub = 11,
    Mul = 12,
    Div = 13,
    Mod = 14,
    Neg = 15,
    Eq = 20,
    Ne = 21,
    Lt = 22,
    Le = 23,
    Gt = 24,
    Ge = 25,
    And = 30,
    Or = 31,
    Not = 32,
    Jump = 40,
    JumpIf = 41,
    JumpIfNot = 42,
    Call = 50,
    Return = 51,
    Halt = 52,
    Loop = 60,
    Break = 61,
    Continue = 62,
    Push = 70,
    Get = 71,
    Set = 72,
    Len = 73,
    Print = 80,
    PrintInt = 81,
    PrintChar = 82,
    StrLen = 90,
    Substring = 91,
    Concat = 92,
    StrCmp = 93,
    Ord = 94,
    Char = 95,
    AgentSpawn = 200,
    AgentHalt = 201,
    AgentDissolve = 202,
    CycleBegin = 220,
    CycleEnd = 221,
    LatentGet = 222,
    LatentSet = 223,
    ScaleCreate = 230,
    ScaleAddAgent = 231,
    ScaleRemoveAgent = 232,
    BarrierCreate = 233,
    BarrierArrive = 234,
    BarrierCheck = 235,
    ConsensusCreate = 236,
    ConsensusVote = 237,
    ConsensusResult = 238,
    GovernCheck = 250,
    GovernRegister = 251,
    GovernSetConfidence = 252,
    GovernSetCycleDepth = 253,
    GovernAdvanceStep = 254,
    EffectCreate = 255,
    TensorCreate = 270,
    TensorFromData = 271,
    TensorGet = 272,
    TensorSet = 273,
    TensorAdd = 274,
    TensorMul = 275,
    TensorMatmul = 276,
    TensorReshape = 277,
    TensorSoftmax = 278,
    TensorRelu = 279,
    RSIPropose = 280,
    RSIVote = 281,
    RSIValidate = 282,
    RSIApply = 283,
    RSIRollback = 284,
    RSIGetStatus = 285,
    MemoryGet = 290,
    MemorySet = 291,
    MemoryAddBehavior = 292,
    MemoryAddWeight = 293,
}

impl Opcode {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            0 => Some(Opcode::Nop),
            1 => Some(Opcode::Const),
            2 => Some(Opcode::Move),
            10 => Some(Opcode::Add),
            11 => Some(Opcode::Sub),
            12 => Some(Opcode::Mul),
            13 => Some(Opcode::Div),
            14 => Some(Opcode::Mod),
            15 => Some(Opcode::Neg),
            20 => Some(Opcode::Eq),
            21 => Some(Opcode::Ne),
            22 => Some(Opcode::Lt),
            23 => Some(Opcode::Le),
            24 => Some(Opcode::Gt),
            25 => Some(Opcode::Ge),
            30 => Some(Opcode::And),
            31 => Some(Opcode::Or),
            32 => Some(Opcode::Not),
            40 => Some(Opcode::Jump),
            41 => Some(Opcode::JumpIf),
            42 => Some(Opcode::JumpIfNot),
            50 => Some(Opcode::Call),
            51 => Some(Opcode::Return),
            52 => Some(Opcode::Halt),
            60 => Some(Opcode::Loop),
            61 => Some(Opcode::Break),
            62 => Some(Opcode::Continue),
            70 => Some(Opcode::Push),
            71 => Some(Opcode::Get),
            72 => Some(Opcode::Set),
            73 => Some(Opcode::Len),
            80 => Some(Opcode::Print),
            81 => Some(Opcode::PrintInt),
            82 => Some(Opcode::PrintChar),
            90 => Some(Opcode::StrLen),
            91 => Some(Opcode::Substring),
            92 => Some(Opcode::Concat),
            93 => Some(Opcode::StrCmp),
            94 => Some(Opcode::Ord),
            95 => Some(Opcode::Char),
            200 => Some(Opcode::AgentSpawn),
            201 => Some(Opcode::AgentHalt),
            202 => Some(Opcode::AgentDissolve),
            220 => Some(Opcode::CycleBegin),
            221 => Some(Opcode::CycleEnd),
            222 => Some(Opcode::LatentGet),
            223 => Some(Opcode::LatentSet),
            230 => Some(Opcode::ScaleCreate),
            231 => Some(Opcode::ScaleAddAgent),
            232 => Some(Opcode::ScaleRemoveAgent),
            233 => Some(Opcode::BarrierCreate),
            234 => Some(Opcode::BarrierArrive),
            235 => Some(Opcode::BarrierCheck),
            236 => Some(Opcode::ConsensusCreate),
            237 => Some(Opcode::ConsensusVote),
            238 => Some(Opcode::ConsensusResult),
            250 => Some(Opcode::GovernCheck),
            251 => Some(Opcode::GovernRegister),
            252 => Some(Opcode::GovernSetConfidence),
            253 => Some(Opcode::GovernSetCycleDepth),
            254 => Some(Opcode::GovernAdvanceStep),
            255 => Some(Opcode::EffectCreate),
            270 => Some(Opcode::TensorCreate),
            271 => Some(Opcode::TensorFromData),
            272 => Some(Opcode::TensorGet),
            273 => Some(Opcode::TensorSet),
            274 => Some(Opcode::TensorAdd),
            275 => Some(Opcode::TensorMul),
            276 => Some(Opcode::TensorMatmul),
            277 => Some(Opcode::TensorReshape),
            278 => Some(Opcode::TensorSoftmax),
            279 => Some(Opcode::TensorRelu),
            280 => Some(Opcode::RSIPropose),
            281 => Some(Opcode::RSIVote),
            282 => Some(Opcode::RSIValidate),
            283 => Some(Opcode::RSIApply),
            284 => Some(Opcode::RSIRollback),
            285 => Some(Opcode::RSIGetStatus),
            290 => Some(Opcode::MemoryGet),
            291 => Some(Opcode::MemorySet),
            292 => Some(Opcode::MemoryAddBehavior),
            293 => Some(Opcode::MemoryAddWeight),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bytecode {
    pub code: Vec<u8>,
    pub constants: Vec<crate::Value>,
    pub strings: Vec<String>,
}

impl Bytecode {
    pub fn new() -> Self {
        Bytecode {
            code: Vec::new(),
            constants: Vec::new(),
            strings: Vec::new(),
        }
    }

    pub fn emit(&mut self, op: Opcode) {
        let v = op as u16;
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    pub fn emit_u8(&mut self, v: u8) {
        self.code.push(v);
    }

    pub fn emit_u32(&mut self, v: u32) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    pub fn emit_i64(&mut self, v: i64) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    pub fn add_constant(&mut self, val: crate::Value) -> u32 {
        let idx = self.constants.len() as u32;
        self.constants.push(val);
        idx
    }

    pub fn add_string(&mut self, s: String) -> u32 {
        let idx = self.strings.len() as u32;
        self.strings.push(s);
        idx
    }

    pub fn read_u8(&self, pc: &mut usize) -> crate::RuntimeResult<u8> {
        if *pc >= self.code.len() {
            return Err(crate::RuntimeError::new("Unexpected end of bytecode", *pc));
        }
        let v = self.code[*pc];
        *pc += 1;
        Ok(v)
    }

    pub fn read_u16(&self, pc: &mut usize) -> crate::RuntimeResult<u16> {
        if *pc + 2 > self.code.len() {
            return Err(crate::RuntimeError::new("Unexpected end of bytecode", *pc));
        }
        let bytes: [u8; 2] = self.code[*pc..*pc + 2].try_into().unwrap();
        *pc += 2;
        Ok(u16::from_le_bytes(bytes))
    }

    pub fn read_u32(&self, pc: &mut usize) -> crate::RuntimeResult<u32> {
        if *pc + 4 > self.code.len() {
            return Err(crate::RuntimeError::new("Unexpected end of bytecode", *pc));
        }
        let bytes: [u8; 4] = self.code[*pc..*pc + 4].try_into().unwrap();
        *pc += 4;
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn read_i64(&self, pc: &mut usize) -> crate::RuntimeResult<i64> {
        if *pc + 8 > self.code.len() {
            return Err(crate::RuntimeError::new("Unexpected end of bytecode", *pc));
        }
        let bytes: [u8; 8] = self.code[*pc..*pc + 8].try_into().unwrap();
        *pc += 8;
        Ok(i64::from_le_bytes(bytes))
    }
}

impl Default for Bytecode {
    fn default() -> Self {
        Self::new()
    }
}

impl Bytecode {
    pub fn serialize(&self) -> Vec<u8> {
        let constants_bytes = self.serialize_constants();
        let strings_bytes = self.serialize_strings();

        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.code);
        hasher.update(&constants_bytes);
        hasher.update(&strings_bytes);
        let hash = hasher.finalize();

        let mut data = Vec::with_capacity(
            HEADER_SIZE + self.code.len() + constants_bytes.len() + strings_bytes.len(),
        );

        data.extend_from_slice(BYTECODE_MAGIC);
        data.extend_from_slice(&BYTECODE_VERSION.to_le_bytes());
        data.extend_from_slice(&(self.code.len() as u32).to_le_bytes());
        data.extend_from_slice(&(self.constants.len() as u32).to_le_bytes());
        data.extend_from_slice(&(self.strings.len() as u32).to_le_bytes());
        data.extend_from_slice(hash.as_bytes());
        data.extend_from_slice(&self.code);
        data.extend_from_slice(&constants_bytes);
        data.extend_from_slice(&strings_bytes);

        data
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, BytecodeError> {
        if data.len() < HEADER_SIZE {
            return Err(BytecodeError::TruncatedData);
        }

        if &data[0..4] != BYTECODE_MAGIC {
            return Err(BytecodeError::InvalidMagic);
        }

        let version = u16::from_le_bytes([data[4], data[5]]);
        if version != BYTECODE_VERSION {
            return Err(BytecodeError::UnsupportedVersion);
        }

        let code_size = u32::from_le_bytes([data[6], data[7], data[8], data[9]]) as usize;
        let constants_count = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
        let strings_count = u32::from_le_bytes([data[14], data[15], data[16], data[17]]) as usize;
        let expected_hash: [u8; 32] = data[18..50]
            .try_into()
            .map_err(|_| BytecodeError::TruncatedData)?;

        let payload_start = HEADER_SIZE;
        let payload_end = payload_start + code_size;

        if data.len() < payload_end {
            return Err(BytecodeError::TruncatedData);
        }

        let mut hasher = blake3::Hasher::new();
        let mut offset = payload_start;

        hasher.update(&data[offset..offset + code_size]);
        offset += code_size;

        let (constants, constants_bytes) =
            Self::deserialize_constants(&data[offset..], constants_count)?;
        hasher.update(&constants_bytes);
        offset += constants_bytes.len();

        let (strings, strings_bytes) = Self::deserialize_strings(&data[offset..], strings_count)?;
        hasher.update(&strings_bytes);

        let computed_hash = hasher.finalize();
        if computed_hash.as_bytes() != &expected_hash {
            return Err(BytecodeError::IntegrityViolation);
        }

        Ok(Bytecode {
            code: data[payload_start..payload_start + code_size].to_vec(),
            constants,
            strings,
        })
    }

    fn serialize_constants(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for val in &self.constants {
            match val {
                crate::Value::I64(n) => {
                    data.push(0);
                    data.extend_from_slice(&n.to_le_bytes());
                }
                crate::Value::F64(n) => {
                    data.push(1);
                    data.extend_from_slice(&n.to_le_bytes());
                }
                crate::Value::Bool(b) => {
                    data.push(2);
                    data.push(if *b { 1 } else { 0 });
                }
                crate::Value::String(s) => {
                    data.push(3);
                    let bytes = s.as_bytes();
                    data.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    data.extend_from_slice(bytes);
                }
                crate::Value::Nil => {
                    data.push(4);
                }
                crate::Value::Void => {
                    data.push(5);
                }
                crate::Value::Array(arr) => {
                    data.push(6);
                    data.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                    for v in arr {
                        let serialized = Self::serialize_single_value(v);
                        data.extend_from_slice(&serialized);
                    }
                }
                crate::Value::Map(map) => {
                    data.push(7);
                    data.extend_from_slice(&(map.len() as u32).to_le_bytes());
                    for (k, v) in map {
                        let k_bytes = k.as_bytes();
                        data.extend_from_slice(&(k_bytes.len() as u32).to_le_bytes());
                        data.extend_from_slice(k_bytes);
                        let v_bytes = Self::serialize_single_value(v);
                        data.extend_from_slice(&v_bytes);
                    }
                }
                crate::Value::Bytes(b) => {
                    data.push(8);
                    data.extend_from_slice(&(b.len() as u32).to_le_bytes());
                    data.extend_from_slice(b);
                }
                crate::Value::Tensor(t) => {
                    data.push(9);
                    data.extend_from_slice(&(t.shape.len() as u32).to_le_bytes());
                    for dim in &t.shape {
                        data.extend_from_slice(&(*dim as u64).to_le_bytes());
                    }
                    data.extend_from_slice(&(t.data.len() as u32).to_le_bytes());
                    for d in &t.data {
                        data.extend_from_slice(&d.to_le_bytes());
                    }
                }
            }
        }
        data
    }

    fn serialize_single_value(val: &crate::Value) -> Vec<u8> {
        let mut data = Vec::new();
        match val {
            crate::Value::I64(n) => {
                data.push(0);
                data.extend_from_slice(&n.to_le_bytes());
            }
            crate::Value::F64(n) => {
                data.push(1);
                data.extend_from_slice(&n.to_le_bytes());
            }
            crate::Value::Bool(b) => {
                data.push(2);
                data.push(if *b { 1 } else { 0 });
            }
            crate::Value::String(s) => {
                data.push(3);
                let bytes = s.as_bytes();
                data.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                data.extend_from_slice(bytes);
            }
            crate::Value::Nil => {
                data.push(4);
            }
            crate::Value::Void => {
                data.push(5);
            }
            _ => {
                data.push(4);
            }
        }
        data
    }

    fn deserialize_constants(
        data: &[u8],
        count: usize,
    ) -> Result<(Vec<crate::Value>, Vec<u8>), BytecodeError> {
        let mut constants = Vec::with_capacity(count);
        let mut offset = 0;

        for _ in 0..count {
            if offset >= data.len() {
                return Err(BytecodeError::TruncatedData);
            }
            let type_tag = data[offset];
            offset += 1;

            match type_tag {
                0 => {
                    if offset + 8 > data.len() {
                        return Err(BytecodeError::TruncatedData);
                    }
                    let n = i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                    constants.push(crate::Value::I64(n));
                    offset += 8;
                }
                1 => {
                    if offset + 8 > data.len() {
                        return Err(BytecodeError::TruncatedData);
                    }
                    let n = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                    constants.push(crate::Value::F64(n));
                    offset += 8;
                }
                2 => {
                    if offset >= data.len() {
                        return Err(BytecodeError::TruncatedData);
                    }
                    constants.push(crate::Value::Bool(data[offset] != 0));
                    offset += 1;
                }
                3 => {
                    if offset + 4 > data.len() {
                        return Err(BytecodeError::TruncatedData);
                    }
                    let len =
                        u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                    offset += 4;
                    if offset + len > data.len() {
                        return Err(BytecodeError::TruncatedData);
                    }
                    let s = String::from_utf8(data[offset..offset + len].to_vec())
                        .map_err(|e| BytecodeError::DeserializationFailed(e.to_string()))?;
                    constants.push(crate::Value::String(s));
                    offset += len;
                }
                4 => constants.push(crate::Value::Nil),
                5 => constants.push(crate::Value::Void),
                6 | 7 | 8 | 9 => {
                    return Err(BytecodeError::DeserializationFailed(
                        "Complex constants not yet supported in deserialize".to_string(),
                    ));
                }
                _ => {
                    return Err(BytecodeError::DeserializationFailed(format!(
                        "Unknown constant type tag: {}",
                        type_tag
                    )));
                }
            }
        }

        Ok((constants, data[..offset].to_vec()))
    }

    fn serialize_strings(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for s in &self.strings {
            let bytes = s.as_bytes();
            data.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            data.extend_from_slice(bytes);
        }
        data
    }

    fn deserialize_strings(
        data: &[u8],
        count: usize,
    ) -> Result<(Vec<String>, Vec<u8>), BytecodeError> {
        let mut strings = Vec::with_capacity(count);
        let mut offset = 0;

        for _ in 0..count {
            if offset + 4 > data.len() {
                return Err(BytecodeError::TruncatedData);
            }
            let len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            if offset + len > data.len() {
                return Err(BytecodeError::TruncatedData);
            }
            let s = String::from_utf8(data[offset..offset + len].to_vec())
                .map_err(|e| BytecodeError::DeserializationFailed(e.to_string()))?;
            strings.push(s);
            offset += len;
        }

        Ok((strings, data[..offset].to_vec()))
    }

    pub fn compute_hash(&self) -> [u8; 32] {
        let serialized = self.serialize();
        let header_end = HEADER_SIZE;
        let hash = blake3::hash(&serialized[header_end..]);
        *hash.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_simple() {
        let mut bc = Bytecode::new();
        bc.emit(Opcode::Nop);
        bc.emit(Opcode::Halt);
        bc.add_constant(crate::Value::I64(42));
        bc.add_constant(crate::Value::F64(3.14));
        bc.add_string("test".to_string());

        let serialized = bc.serialize();
        let deserialized = Bytecode::deserialize(&serialized).unwrap();

        assert_eq!(bc.code, deserialized.code);
        assert_eq!(bc.constants.len(), deserialized.constants.len());
        assert_eq!(bc.strings, deserialized.strings);
    }

    #[test]
    fn test_invalid_magic_rejected() {
        let bad_data = b"BAD!\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let result = Bytecode::deserialize(bad_data);
        assert!(matches!(result, Err(BytecodeError::InvalidMagic)));
    }

    #[test]
    fn test_truncated_rejected() {
        let truncated = b"LC-B";
        let result = Bytecode::deserialize(truncated);
        assert!(matches!(result, Err(BytecodeError::TruncatedData)));
    }

    #[test]
    fn test_tampered_rejected() {
        let mut bc = Bytecode::new();
        bc.emit(Opcode::Nop);
        bc.emit(Opcode::Nop);
        bc.emit(Opcode::Nop);
        bc.emit(Opcode::Nop);
        bc.emit(Opcode::Halt);
        bc.add_constant(crate::Value::I64(12345));
        bc.add_string("test_string".to_string());

        let mut serialized = bc.serialize();
        assert!(
            serialized.len() > HEADER_SIZE + 10,
            "Serialized bytecode too short for test"
        );

        serialized[HEADER_SIZE + 5] ^= 0xFF;

        let result = Bytecode::deserialize(&serialized);
        assert!(matches!(result, Err(BytecodeError::IntegrityViolation)));
    }

    #[test]
    fn test_hash_changes_with_content() {
        let mut bc1 = Bytecode::new();
        bc1.emit(Opcode::Nop);
        bc1.emit(Opcode::Halt);

        let mut bc2 = Bytecode::new();
        bc2.emit(Opcode::Nop);
        bc2.emit(Opcode::Nop);
        bc2.emit(Opcode::Halt);

        let hash1 = bc1.compute_hash();
        let hash2 = bc2.compute_hash();

        assert_ne!(hash1, hash2);
    }
}
