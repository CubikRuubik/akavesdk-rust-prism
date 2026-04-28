use alloy::sol;
use alloy::sol_types::SolError;

// All custom errors declared in the storage contract ABI.
sol! {
    error AddressEmptyCode(address target);
    error BlockAlreadyExists();
    error BlockAlreadyFilled();
    error BlockInvalid();
    error BlockNonexists();
    error BucketAlreadyExists();
    error BucketInvalid();
    error BucketInvalidOwner();
    error BucketNonempty();
    error BucketNonexists();
    error ChunkCIDMismatch(bytes fileCID);
    error ECDSAInvalidSignature();
    error ECDSAInvalidSignatureLength(uint256 length);
    error ECDSAInvalidSignatureS(bytes32 s);
    error ERC1967InvalidImplementation(address implementation);
    error ERC1967NonPayable();
    error FailedCall();
    error FileAlreadyExists();
    error FileChunkDuplicate();
    error FileFullyUploaded();
    error FileInvalid();
    error FileNameDuplicate();
    error FileNonempty();
    error FileNotExists();
    error FileNotFilled();
    error IndexMismatch();
    error InvalidArrayLength(uint256 cidsLength, uint256 sizesLength);
    error InvalidBlockIndex();
    error InvalidBlocksAmount();
    error InvalidEncodedSize();
    error InvalidFileBlocksCount();
    error InvalidFileCID();
    error InvalidImplementation();
    error InvalidInitialization();
    error InvalidLastBlockSize();
    error InvalidPeerIndex();
    error LastChunkDuplicate();
    error NoPeersForCID();
    error NotEligibleToUpgrade();
    error NotInitializing();
    error OffsetOutOfBounds();
    error TooManyBlockCIDs();
    error UUPSUnauthorizedCallContext();
    error UUPSUnsupportedProxiableUUID(bytes32 slot);
    error WrongAuthority();
}

/// Match the first 4 bytes of `data` against every known error selector and
/// return the error name, or `None` if unrecognised.
pub fn decode_revert_reason(data: &[u8]) -> Option<String> {
    if data.len() < 4 {
        return None;
    }
    let sel = &data[..4];

    macro_rules! match_selectors {
        ($($ty:ident),* $(,)?) => {
            $(if sel == $ty::SELECTOR { return Some(stringify!($ty).to_string()); })*
        }
    }

    match_selectors!(
        AddressEmptyCode,
        BlockAlreadyExists,
        BlockAlreadyFilled,
        BlockInvalid,
        BlockNonexists,
        BucketAlreadyExists,
        BucketInvalid,
        BucketInvalidOwner,
        BucketNonempty,
        BucketNonexists,
        ChunkCIDMismatch,
        ECDSAInvalidSignature,
        ECDSAInvalidSignatureLength,
        ECDSAInvalidSignatureS,
        ERC1967InvalidImplementation,
        ERC1967NonPayable,
        FailedCall,
        FileAlreadyExists,
        FileChunkDuplicate,
        FileFullyUploaded,
        FileInvalid,
        FileNameDuplicate,
        FileNonempty,
        FileNotExists,
        FileNotFilled,
        IndexMismatch,
        InvalidArrayLength,
        InvalidBlockIndex,
        InvalidBlocksAmount,
        InvalidEncodedSize,
        InvalidFileBlocksCount,
        InvalidFileCID,
        InvalidImplementation,
        InvalidInitialization,
        InvalidLastBlockSize,
        InvalidPeerIndex,
        LastChunkDuplicate,
        NoPeersForCID,
        NotEligibleToUpgrade,
        NotInitializing,
        OffsetOutOfBounds,
        TooManyBlockCIDs,
        UUPSUnauthorizedCallContext,
        UUPSUnsupportedProxiableUUID,
        WrongAuthority,
    );

    // Standard `Error(string)` revert: selector 0x08c379a0
    if data.starts_with(&[0x08, 0xc3, 0x79, 0xa0]) && data.len() >= 68 {
        // ABI layout: selector(4) + offset(32) + length(32) + utf8_bytes(...)
        let len = u32::from_be_bytes([data[64], data[65], data[66], data[67]]) as usize;
        if data.len() >= 68 + len {
            if let Ok(s) = std::str::from_utf8(&data[68..68 + len]) {
                return Some(format!("revert: {s}"));
            }
        }
    }

    None
}

/// Fallback: some nodes (Anvil) include the decoded name in the RPC message,
/// e.g. `"execution reverted: BucketAlreadyExists()"`.
pub fn extract_error_from_message(msg: &str) -> Option<String> {
    let rest = msg.strip_prefix("execution reverted: ")?;
    Some(rest.trim_end_matches("()").to_string())
}
