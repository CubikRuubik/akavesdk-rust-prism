use crate::types::sdk_types::AkaveError;

#[cfg(not(target_arch = "wasm32"))]
pub async fn range_download(
    client: &reqwest::Client,
    url: &str,
    offset: i64,
    length: i64,
) -> Result<Vec<u8>, AkaveError> {
    if length <= 0 {
        return Err(AkaveError::InvalidInput("length must be positive".into()));
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
        .map_err(|e| AkaveError::Transient(e.to_string()))?;

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
        .map_err(|e| AkaveError::Transient(e.to_string()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_range_download_rejects_negative_offset() {
        let client = reqwest::Client::new();
        let result = range_download(&client, "http://localhost:1", -1, 10).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("offset"), "expected offset error, got: {msg}");
    }

    #[tokio::test]
    async fn test_range_download_rejects_zero_length() {
        let client = reqwest::Client::new();
        let result = range_download(&client, "http://localhost:1", 0, 0).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("length"), "expected length error, got: {msg}");
    }

    #[tokio::test]
    async fn test_range_download_rejects_negative_length() {
        let client = reqwest::Client::new();
        let result = range_download(&client, "http://localhost:1", 0, -5).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("length"), "expected length error, got: {msg}");
    }
}
