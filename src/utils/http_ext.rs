use crate::types::sdk_types::AkaveError;

#[cfg(not(target_arch = "wasm32"))]
pub async fn range_download(
    client: &reqwest::Client,
    url: &str,
    offset: i64,
    length: i64,
) -> Result<Vec<u8>, AkaveError> {
    if length <= 0 {
        return Err(AkaveError::InvalidInput(
            "length must be positive".into(),
        ));
    }
    if offset < 0 {
        return Err(AkaveError::InvalidInput(
            "offset must be non-negative".into(),
        ));
    }
    let end = offset + length - 1;
    let range_header = format!("bytes={}-{}", offset, end);
    let resp = client
        .get(url)
        .header("Range", range_header)
        .send()
        .await
        .map_err(|e| AkaveError::InternalError(e.to_string()))?;

    let status = resp.status().as_u16();
    if status != 206 && status != 200 {
        return Err(AkaveError::InternalError(format!(
            "range download failed with status {}",
            status
        )));
    }

    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| AkaveError::InternalError(e.to_string()))
}
