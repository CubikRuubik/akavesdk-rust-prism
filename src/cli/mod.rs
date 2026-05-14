use std::sync::Arc;

use clap::{Args, Parser, Subcommand};
use web3::{
    signing::{Key, SecretKey, SecretKeyRef},
    types::U256,
};

use crate::AkaveSDKBuilder;

// ── CLI structure ─────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "akave")]
struct Cli {
    #[arg(long, global = true, default_value = "")]
    node_address: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Bucket {
        #[command(subcommand)]
        action: BucketAction,
    },
    File {
        #[command(subcommand)]
        action: FileAction,
    },
    Wallet {
        #[command(subcommand)]
        action: WalletAction,
    },
}

#[derive(Subcommand)]
enum BucketAction {
    Create(BucketCreateArgs),
    Delete(BucketDeleteArgs),
    View(BucketViewArgs),
    List(BucketListArgs),
}

#[derive(Subcommand)]
enum FileAction {
    Upload(FileUploadArgs),
    Download(FileDownloadArgs),
    Delete(FileDeleteArgs),
    List(FileListArgs),
    Info(FileInfoArgs),
    ArchivalMetadata(FileArchivalMetadataArgs),
}

#[derive(Subcommand)]
enum WalletAction {
    Import(WalletImportArgs),
    Balance(WalletBalanceArgs),
    Create(WalletCreateArgs),
    List(WalletListArgs),
    ExportKey(WalletExportKeyArgs),
}

// ── Argument structs ──────────────────────────────────────────────────────────

#[derive(Args)]
struct BucketCreateArgs {
    bucket_name: String,
    #[arg(long)]
    private_key: String,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    metadata_encryption: bool,
    #[arg(long)]
    encryption_key: Option<String>,
}

#[derive(Args)]
struct BucketDeleteArgs {
    bucket_name: String,
    #[arg(long)]
    private_key: String,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    metadata_encryption: bool,
    #[arg(long)]
    encryption_key: Option<String>,
}

#[derive(Args)]
struct BucketViewArgs {
    bucket_name: String,
    #[arg(long)]
    private_key: String,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    metadata_encryption: bool,
    #[arg(long)]
    encryption_key: Option<String>,
}

#[derive(Args)]
struct BucketListArgs {
    #[arg(long)]
    private_key: String,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    metadata_encryption: bool,
    #[arg(long)]
    encryption_key: Option<String>,
    #[arg(long, default_value_t = 0)]
    offset: i64,
    #[arg(long, default_value_t = 20)]
    limit: i64,
}

#[derive(Args)]
struct FileUploadArgs {
    bucket_name: String,
    file_path: String,
    #[arg(long)]
    private_key: String,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    metadata_encryption: bool,
    #[arg(long)]
    encryption_key: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    disable_erasure_coding: bool,
}

#[derive(Args)]
struct FileDownloadArgs {
    bucket_name: String,
    file_name: String,
    dest_path: String,
    #[arg(long)]
    private_key: String,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    metadata_encryption: bool,
    #[arg(long, short = 'e')]
    encryption_key: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    disable_erasure_coding: bool,
}

#[derive(Args)]
struct FileDeleteArgs {
    bucket_name: String,
    file_name: String,
    #[arg(long)]
    private_key: String,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    metadata_encryption: bool,
    #[arg(long)]
    encryption_key: Option<String>,
}

#[derive(Args)]
struct FileListArgs {
    bucket_name: String,
    #[arg(long)]
    private_key: String,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    metadata_encryption: bool,
    #[arg(long)]
    encryption_key: Option<String>,
    #[arg(long, default_value_t = 0)]
    offset: i64,
    #[arg(long, default_value_t = 20)]
    limit: i64,
}

#[derive(Args)]
struct FileInfoArgs {
    bucket_name: String,
    file_name: String,
    #[arg(long)]
    private_key: String,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    metadata_encryption: bool,
    #[arg(long)]
    encryption_key: Option<String>,
}

#[derive(Args)]
struct FileArchivalMetadataArgs {
    bucket_name: String,
    file_name: String,
    #[arg(long)]
    private_key: String,
    #[arg(short = 'v', long, action = clap::ArgAction::SetTrue)]
    verbose: bool,
}

#[derive(Args)]
struct WalletImportArgs {
    name: String,
    private_key: String,
    #[arg(long)]
    keystore: Option<String>,
}

#[derive(Args)]
struct WalletBalanceArgs {
    name: Option<String>,
    #[arg(long)]
    keystore: Option<String>,
}

#[derive(Args)]
struct WalletCreateArgs {
    name: String,
    #[arg(long)]
    keystore: Option<String>,
}

#[derive(Args)]
struct WalletListArgs {
    #[arg(long)]
    keystore: Option<String>,
}

#[derive(Args)]
struct WalletExportKeyArgs {
    name: String,
    #[arg(long)]
    keystore: Option<String>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Run CLI args without the program name, e.g. `&["bucket", "create", "--private-key", ...]`.
/// Returns (stdout, stderr, success) — mirrors Go's captureCobraOutput pattern.
pub async fn run_from_args(args: &[&str]) -> (String, String, bool) {
    let full: Vec<&str> = std::iter::once("akave")
        .chain(args.iter().copied())
        .collect();
    match Cli::try_parse_from(full) {
        Err(e) => (String::new(), e.to_string(), false),
        Ok(cli) => {
            let node = cli.node_address;
            match cli.command {
                Commands::Bucket {
                    action: BucketAction::Create(a),
                } => bucket_create(&node, a).await,
                Commands::Bucket {
                    action: BucketAction::Delete(a),
                } => bucket_delete(&node, a).await,
                Commands::Bucket {
                    action: BucketAction::View(a),
                } => bucket_view(&node, a).await,
                Commands::Bucket {
                    action: BucketAction::List(a),
                } => bucket_list(&node, a).await,
                Commands::File {
                    action: FileAction::Upload(a),
                } => file_upload(&node, a).await,
                Commands::File {
                    action: FileAction::Download(a),
                } => file_download(&node, a).await,
                Commands::File {
                    action: FileAction::Delete(a),
                } => file_delete(&node, a).await,
                Commands::File {
                    action: FileAction::List(a),
                } => file_list(&node, a).await,
                Commands::File {
                    action: FileAction::Info(a),
                } => file_info(&node, a).await,
                Commands::File {
                    action: FileAction::ArchivalMetadata(a),
                } => file_archival_metadata(&node, a).await,
                Commands::Wallet {
                    action: WalletAction::Import(a),
                } => wallet_import(a),
                Commands::Wallet {
                    action: WalletAction::Balance(a),
                } => wallet_balance(&node, a).await,
                Commands::Wallet {
                    action: WalletAction::Create(a),
                } => wallet_create(a),
                Commands::Wallet {
                    action: WalletAction::List(a),
                } => wallet_list(a),
                Commands::Wallet {
                    action: WalletAction::ExportKey(a),
                } => wallet_export_key(a),
            }
        }
    }
}

// ── SDK builder helper ────────────────────────────────────────────────────────

fn make_builder(
    node_address: &str,
    private_key: &str,
    metadata_encryption: bool,
    encryption_key: Option<&str>,
    erasure_coding: bool,
) -> AkaveSDKBuilder {
    let mut b = AkaveSDKBuilder::new(node_address).with_private_key(private_key);
    // Only set default encryption when metadata encryption is explicitly requested.
    // Per-file content encryption is handled separately via the passwd parameter.
    if metadata_encryption {
        if let Some(key) = encryption_key {
            b = b.with_default_encryption(key, true);
        }
    }
    if erasure_coding {
        b = b.with_erasure_coding(3, 2);
    }
    b
}

// ── Bucket handlers ───────────────────────────────────────────────────────────

async fn bucket_create(node: &str, a: BucketCreateArgs) -> (String, String, bool) {
    if a.bucket_name.is_empty() {
        return (String::new(), "bucket name is required".to_string(), false);
    }
    let sdk = match make_builder(
        node,
        &a.private_key,
        a.metadata_encryption,
        a.encryption_key.as_deref(),
        false,
    )
    .build()
    .await
    {
        Ok(s) => s,
        Err(e) => return (String::new(), format!("{e}"), false),
    };
    match sdk.create_bucket(&a.bucket_name).await {
        Ok(resp) => (
            format!("Bucket created: Name={}\n", resp.name),
            String::new(),
            true,
        ),
        Err(e) => (String::new(), format!("{e}"), false),
    }
}

async fn bucket_delete(node: &str, a: BucketDeleteArgs) -> (String, String, bool) {
    if a.bucket_name.is_empty() {
        return (String::new(), "bucket name is required".to_string(), false);
    }
    let sdk = match make_builder(
        node,
        &a.private_key,
        a.metadata_encryption,
        a.encryption_key.as_deref(),
        false,
    )
    .build()
    .await
    {
        Ok(s) => s,
        Err(e) => return (String::new(), format!("{e}"), false),
    };
    match sdk.delete_bucket(&a.bucket_name).await {
        Ok(_) => (
            format!("Bucket deleted: Name={}\n", a.bucket_name),
            String::new(),
            true,
        ),
        Err(e) => (String::new(), format!("{e}"), false),
    }
}

async fn bucket_view(node: &str, a: BucketViewArgs) -> (String, String, bool) {
    if a.bucket_name.is_empty() {
        return (String::new(), "bucket name is required".to_string(), false);
    }
    let sdk = match make_builder(
        node,
        &a.private_key,
        a.metadata_encryption,
        a.encryption_key.as_deref(),
        false,
    )
    .build()
    .await
    {
        Ok(s) => s,
        Err(e) => return (String::new(), format!("{e}"), false),
    };
    match sdk.view_bucket(&a.bucket_name).await {
        Ok(b) => (
            format!("Name={}, CreatedAt={}\n", b.name, b.created_at),
            String::new(),
            true,
        ),
        Err(e) => (String::new(), format!("{e}"), false),
    }
}

async fn bucket_list(node: &str, a: BucketListArgs) -> (String, String, bool) {
    let sdk = match make_builder(
        node,
        &a.private_key,
        a.metadata_encryption,
        a.encryption_key.as_deref(),
        false,
    )
    .build()
    .await
    {
        Ok(s) => s,
        Err(e) => return (String::new(), format!("{e}"), false),
    };
    match sdk.list_buckets(a.offset, a.limit).await {
        Ok(resp) => {
            if resp.buckets.is_empty() {
                return ("No buckets\n".to_string(), String::new(), true);
            }
            let out = resp
                .buckets
                .iter()
                .map(|b| format!("Bucket: Name={}, CreatedAt={}\n", b.name, b.created_at))
                .collect::<String>();
            (out, String::new(), true)
        }
        Err(e) => (String::new(), format!("{e}"), false),
    }
}

// ── File handlers ─────────────────────────────────────────────────────────────

async fn file_upload(node: &str, a: FileUploadArgs) -> (String, String, bool) {
    if a.bucket_name.is_empty() {
        return (String::new(), "bucket name is required".to_string(), false);
    }
    if a.file_path.is_empty() {
        return (String::new(), "file path is required".to_string(), false);
    }

    let file_name = match std::path::Path::new(&a.file_path).file_name() {
        Some(n) => n.to_string_lossy().to_string(),
        None => return (String::new(), "invalid file path".to_string(), false),
    };

    let mut file = match std::fs::File::open(&a.file_path) {
        Ok(f) => f,
        Err(e) => return (String::new(), format!("failed to open file: {e}"), false),
    };

    let sdk = match make_builder(
        node,
        &a.private_key,
        a.metadata_encryption,
        a.encryption_key.as_deref(),
        !a.disable_erasure_coding,
    )
    .build()
    .await
    {
        Ok(s) => s,
        Err(e) => return (String::new(), format!("{e}"), false),
    };

    let passwd = a
        .encryption_key
        .as_deref()
        .filter(|k| !k.is_empty())
        .filter(|_| !a.metadata_encryption);
    match sdk
        .upload_file(&a.bucket_name, &file_name, &mut file, passwd)
        .await
    {
        Ok(_) => {
            let size_str = match sdk.view_file_info(&a.bucket_name, &file_name).await {
                Ok(info) => format!(
                    ", ActualSize={}, EncodedSize={}",
                    info.actual_size, info.encoded_size
                ),
                Err(_) => String::new(),
            };
            (
                format!(
                    "File uploaded successfully: Name={}{}\n",
                    file_name, size_str
                ),
                String::new(),
                true,
            )
        }
        Err(e) => (String::new(), format!("{e}"), false),
    }
}

async fn file_download(node: &str, a: FileDownloadArgs) -> (String, String, bool) {
    if a.bucket_name.is_empty() {
        return (String::new(), "bucket name is required".to_string(), false);
    }
    if a.file_name.is_empty() {
        return (String::new(), "file name is required".to_string(), false);
    }
    if a.dest_path.is_empty() {
        return (
            String::new(),
            "destination path is required".to_string(),
            false,
        );
    }

    let dest = std::path::Path::new(&a.dest_path).join(&a.file_name);
    let out_file = match std::fs::File::create(&dest) {
        Ok(f) => f,
        Err(e) => {
            return (
                String::new(),
                format!("failed to create destination file: {e}"),
                false,
            )
        }
    };

    let sdk = match make_builder(
        node,
        &a.private_key,
        a.metadata_encryption,
        a.encryption_key.as_deref(),
        !a.disable_erasure_coding,
    )
    .build()
    .await
    {
        Ok(s) => s,
        Err(e) => return (String::new(), format!("{e}"), false),
    };

    let passwd = a
        .encryption_key
        .as_deref()
        .filter(|k| !k.is_empty())
        .filter(|_| !a.metadata_encryption);
    match Arc::new(sdk)
        .download_file(&a.bucket_name, &a.file_name, passwd, out_file)
        .await
    {
        Ok(_) => (
            format!("File downloaded successfully: Name={}\n", a.file_name),
            String::new(),
            true,
        ),
        Err(e) => (String::new(), format!("{e}"), false),
    }
}

async fn file_delete(node: &str, a: FileDeleteArgs) -> (String, String, bool) {
    if a.bucket_name.is_empty() {
        return (String::new(), "bucket name is required".to_string(), false);
    }
    if a.file_name.is_empty() {
        return (String::new(), "file name is required".to_string(), false);
    }
    let sdk = match make_builder(
        node,
        &a.private_key,
        a.metadata_encryption,
        a.encryption_key.as_deref(),
        false,
    )
    .build()
    .await
    {
        Ok(s) => s,
        Err(e) => return (String::new(), format!("{e}"), false),
    };
    match sdk.delete_file(&a.bucket_name, &a.file_name).await {
        Ok(_) => (
            format!("File successfully deleted: Name={}\n", a.file_name),
            String::new(),
            true,
        ),
        Err(e) => (String::new(), format!("{e}"), false),
    }
}

async fn file_list(node: &str, a: FileListArgs) -> (String, String, bool) {
    if a.bucket_name.is_empty() {
        return (String::new(), "bucket name is required".to_string(), false);
    }
    let sdk = match make_builder(
        node,
        &a.private_key,
        a.metadata_encryption,
        a.encryption_key.as_deref(),
        false,
    )
    .build()
    .await
    {
        Ok(s) => s,
        Err(e) => return (String::new(), format!("{e}"), false),
    };
    match sdk.list_files(&a.bucket_name, a.offset, a.limit).await {
        Ok(resp) => {
            if resp.files.is_empty() {
                return ("No files\n".to_string(), String::new(), true);
            }
            let out = resp
                .files
                .iter()
                .map(|f| {
                    format!(
                        "File: Name={}, ActualSize={}, EncodedSize={}\n",
                        f.name, f.actual_size, f.encoded_size
                    )
                })
                .collect::<String>();
            (out, String::new(), true)
        }
        Err(e) => (String::new(), format!("{e}"), false),
    }
}

async fn file_info(node: &str, a: FileInfoArgs) -> (String, String, bool) {
    if a.bucket_name.is_empty() {
        return (String::new(), "bucket name is required".to_string(), false);
    }
    if a.file_name.is_empty() {
        return (String::new(), "file name is required".to_string(), false);
    }
    let sdk = match make_builder(
        node,
        &a.private_key,
        a.metadata_encryption,
        a.encryption_key.as_deref(),
        false,
    )
    .build()
    .await
    {
        Ok(s) => s,
        Err(e) => return (String::new(), format!("{e}"), false),
    };
    match sdk.view_file_info(&a.bucket_name, &a.file_name).await {
        Ok(info) => (
            format!(
                "File: Name={}, RootCID={}, ActualSize={}, EncodedSize={}, IsPublic={}\n",
                info.name, info.root_cid, info.actual_size, info.encoded_size, info.is_public
            ),
            String::new(),
            true,
        ),
        Err(e) => (String::new(), format!("{e}"), false),
    }
}

// ── Archival metadata handler ─────────────────────────────────────────────────

async fn file_archival_metadata(node: &str, a: FileArchivalMetadataArgs) -> (String, String, bool) {
    if a.bucket_name.trim().is_empty() {
        return (String::new(), "bucket name is required".to_string(), false);
    }
    if a.file_name.trim().is_empty() {
        return (String::new(), "file name is required".to_string(), false);
    }
    let sdk = match make_builder(node, &a.private_key, false, None, false)
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => return (String::new(), format!("{e}"), false),
    };
    match sdk.archival_metadata(&a.bucket_name, &a.file_name).await {
        Ok(metadata) => {
            let out = if a.verbose {
                let mut s = format!(
                    "Bucket: {}, File: {}\nTotal Chunks: {}\n\n",
                    metadata.bucket_name,
                    metadata.name,
                    metadata.chunks.len()
                );
                for chunk in &metadata.chunks {
                    s.push_str(&format!(
                        "  Chunk CID: {}, Size: {}\n",
                        chunk.cid, chunk.size
                    ));
                    for block in &chunk.blocks {
                        if let Some(pdp) = &block.pdp_data {
                            s.push_str(&format!(
                                "    Block CID: {}, URL: {}, Offset: {}, Size: {}, Dataset ID: {}\n",
                                block.cid, pdp.url, pdp.offset, pdp.size, pdp.data_set_id
                            ));
                        } else {
                            s.push_str(&format!(
                                "    Block CID: {}, Size: {} (No PDP data)\n",
                                block.cid, block.size
                            ));
                        }
                    }
                    s.push('\n');
                }
                s
            } else {
                let all_have_pdp = metadata
                    .chunks
                    .iter()
                    .all(|c| c.blocks.iter().all(|b| b.pdp_data.is_some()));
                if all_have_pdp {
                    "Status: Available for download from archival storage (all blocks have PDP data)\n".to_string()
                } else {
                    "Status: Not fully available in archival storage (some blocks are missing PDP data)\n".to_string()
                }
            };
            (out, String::new(), true)
        }
        Err(e) => (String::new(), format!("{e}"), false),
    }
}

// ── Wallet handlers ───────────────────────────────────────────────────────────

fn default_keystore_dir() -> String {
    match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        Ok(home) => format!("{home}/.akave_wallets"),
        Err(_) => ".akave_wallets".to_string(),
    }
}

fn load_wallet(keystore: &str, name: Option<&str>) -> Result<(String, String, String), String> {
    let wallet_name = match name {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => {
            let entries = std::fs::read_dir(keystore)
                .map_err(|_| "failed to read wallet file".to_string())?;
            let mut found = None;
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.ends_with(".json") {
                    found = Some(fname.trim_end_matches(".json").to_string());
                    break;
                }
            }
            found.ok_or_else(|| "failed to read wallet file".to_string())?
        }
    };
    let wallet_path = format!("{keystore}/{wallet_name}.json");
    let data = std::fs::read_to_string(&wallet_path)
        .map_err(|_| "failed to read wallet file".to_string())?;
    let info: serde_json::Value =
        serde_json::from_str(&data).map_err(|e| format!("failed to parse wallet file: {e}"))?;
    let private_key = info["private_key"]
        .as_str()
        .ok_or("failed to parse wallet file")?
        .to_string();
    let address = info["address"]
        .as_str()
        .ok_or("failed to parse wallet file")?
        .to_string();
    Ok((private_key, address, wallet_name))
}

fn wallet_import(a: WalletImportArgs) -> (String, String, bool) {
    use std::str::FromStr;
    let keystore = a.keystore.unwrap_or_else(default_keystore_dir);
    let wallet_path = format!("{keystore}/{}.json", a.name);
    if std::path::Path::new(&wallet_path).exists() {
        return (
            String::new(),
            format!("wallet with name '{}' already exists", a.name),
            false,
        );
    }
    let key = match SecretKey::from_str(a.private_key.trim_start_matches("0x")) {
        Ok(k) => k,
        Err(e) => return (String::new(), format!("invalid private key: {e}"), false),
    };
    let address = format!("{:?}", SecretKeyRef::new(&key).address());
    let pk_hex = a.private_key.trim_start_matches("0x").to_string();
    let info = serde_json::json!({"address": address, "private_key": pk_hex});
    if let Err(e) = std::fs::create_dir_all(&keystore) {
        return (
            String::new(),
            format!("failed to create keystore directory: {e}"),
            false,
        );
    }
    if let Err(e) = std::fs::write(&wallet_path, info.to_string()) {
        return (
            String::new(),
            format!("failed to write wallet file: {e}"),
            false,
        );
    }
    (
        format!(
            "Wallet imported successfully:\nName: {}\nAddress: {address}\n",
            a.name
        ),
        String::new(),
        true,
    )
}

async fn wallet_balance(node: &str, a: WalletBalanceArgs) -> (String, String, bool) {
    let keystore = a.keystore.unwrap_or_else(default_keystore_dir);
    let (private_key, address, name) = match load_wallet(&keystore, a.name.as_deref()) {
        Ok(r) => r,
        Err(e) => return (String::new(), e, false),
    };
    let sdk = match AkaveSDKBuilder::new(node)
        .with_private_key(&private_key)
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => return (String::new(), format!("{e}"), false),
    };
    match sdk.get_balance().await {
        Ok(balance) => {
            let wei_per_ether = U256::exp10(18);
            let whole = balance / wei_per_ether;
            let remainder = balance % wei_per_ether;
            let frac_4 = (remainder.low_u128() * 10_000) / 1_000_000_000_000_000_000_u128;
            let akvt_str = if frac_4 == 0 {
                format!("{whole}")
            } else {
                let s = format!("{whole}.{frac_4:04}");
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            };
            (
                format!("Wallet:  {name}\nAddress: {address}\nBalance: {akvt_str} AKVT\n"),
                String::new(),
                true,
            )
        }
        Err(e) => (String::new(), format!("{e}"), false),
    }
}

fn wallet_create(a: WalletCreateArgs) -> (String, String, bool) {
    use std::str::FromStr;
    let keystore = a.keystore.unwrap_or_else(default_keystore_dir);
    let wallet_path = format!("{keystore}/{}.json", a.name);
    if std::path::Path::new(&wallet_path).exists() {
        return (
            String::new(),
            format!("wallet '{}' already exists", a.name),
            false,
        );
    }
    let mut secret_bytes = [0u8; 32];
    if let Err(e) = getrandom::getrandom(&mut secret_bytes) {
        return (String::new(), format!("failed to generate key: {e}"), false);
    }
    let pk_hex = hex::encode(secret_bytes);
    let key = match SecretKey::from_str(&pk_hex) {
        Ok(k) => k,
        Err(e) => return (String::new(), format!("invalid generated key: {e}"), false),
    };
    let address = format!("{:?}", SecretKeyRef::new(&key).address());
    let info = serde_json::json!({"address": address, "private_key": pk_hex});
    if let Err(e) = std::fs::create_dir_all(&keystore) {
        return (
            String::new(),
            format!("failed to create keystore directory: {e}"),
            false,
        );
    }
    if let Err(e) = std::fs::write(&wallet_path, info.to_string()) {
        return (
            String::new(),
            format!("failed to write wallet file: {e}"),
            false,
        );
    }
    (
        format!(
            "Wallet ({}) created successfully\nAddress: {address}\n",
            a.name
        ),
        String::new(),
        true,
    )
}

fn wallet_list(a: WalletListArgs) -> (String, String, bool) {
    let keystore = a.keystore.unwrap_or_else(default_keystore_dir);
    let entries = match std::fs::read_dir(&keystore) {
        Ok(e) => e,
        Err(_) => return (String::new(), "no wallets found".to_string(), false),
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let fname = e.file_name().to_string_lossy().to_string();
            fname
                .ends_with(".json")
                .then(|| fname.trim_end_matches(".json").to_string())
        })
        .collect();
    names.sort();
    if names.is_empty() {
        return (String::new(), "no wallets found".to_string(), false);
    }
    (names.join("\n") + "\n", String::new(), true)
}

fn wallet_export_key(a: WalletExportKeyArgs) -> (String, String, bool) {
    let keystore = a.keystore.unwrap_or_else(default_keystore_dir);
    match load_wallet(&keystore, Some(&a.name)) {
        Ok((pk, _, _)) => (format!("Private key: {pk}\n"), String::new(), true),
        Err(e) => (String::new(), e, false),
    }
}

#[cfg(test)]
mod tests {
    use super::run_from_args;

    const NODE_ADDRESS: &str = "http://127.0.0.1:5000";
    const ENC_KEY: &str = "1234567890123456789012345678901212345678901234567890123456789012";
    const FILE_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

    fn private_key() -> String {
        std::env::var("AKAVE_PRIVATE_KEY").unwrap_or_else(|_| {
            "0000000000000000000000000000000000000000000000000000000000000001".to_string()
        })
    }

    fn random_name(prefix: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{}-{:012x}", prefix, nanos % 0x1_0000_0000_0000)
    }

    fn create_temp_file(size: usize) -> (String, String) {
        let name = random_name("akave-test") + ".bin";
        let path = std::env::temp_dir().join(&name);
        std::fs::write(&path, vec![b'x'; size]).unwrap();
        (path.to_string_lossy().into_owned(), name)
    }

    fn create_temp_dir() -> String {
        let name = random_name("akave-dl");
        let path = std::env::temp_dir().join(&name);
        std::fs::create_dir_all(&path).unwrap();
        path.to_string_lossy().into_owned()
    }

    struct TestCase {
        name: &'static str,
        args: Vec<String>,
        expected_output: Vec<String>,
        expect_error: bool,
    }

    async fn run_cases(cases: Vec<TestCase>) {
        for tc in cases {
            let args: Vec<&str> = tc.args.iter().map(|s| s.as_str()).collect();
            let (stdout, stderr, success) = run_from_args(&args).await;
            let combined = stdout + &stderr;
            if tc.expect_error {
                assert!(
                    !success,
                    "[{}] expected error but succeeded. Output: {combined}",
                    tc.name
                );
            } else {
                assert!(
                    success,
                    "[{}] unexpected failure. Output: {combined}",
                    tc.name
                );
            }
            for expected in &tc.expected_output {
                assert!(
                    combined.contains(expected.as_str()),
                    "[{}] expected {:?} in output but got: {combined}",
                    tc.name,
                    expected
                );
            }
        }
    }

    // ── TestExternalCreateBucketCommand ──────────────────────────────────────────

    #[tokio::test]
    async fn test_external_create_bucket_command() {
        let pk = private_key();
        let bucket = random_name("tcb");

        let cases = vec![
            TestCase {
                name: "Create bucket successfully",
                args: vec![
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "bucket".into(),
                    "create".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                ],
                expected_output: vec!["Bucket created".into(), format!("Name={bucket}")],
                expect_error: false,
            },
            TestCase {
                name: "Create bucket successfully with metadata encryption",
                args: vec![
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "bucket".into(),
                    "create".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "--metadata-encryption".into(),
                    "--encryption-key".into(),
                    ENC_KEY.into(),
                    bucket.clone(),
                ],
                expected_output: vec!["Bucket created".into(), format!("Name={bucket}")],
                expect_error: false,
            },
            TestCase {
                name: "Create bucket already exists",
                args: vec![
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "bucket".into(),
                    "create".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                ],
                expected_output: vec!["BucketAlreadyExists".into()],
                expect_error: true,
            },
            TestCase {
                name: "Empty bucket name provided",
                args: vec![
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "bucket".into(),
                    "create".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "".into(),
                ],
                expected_output: vec!["bucket name is required".into()],
                expect_error: true,
            },
            TestCase {
                name: "Invalid private key provided",
                args: vec![
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "bucket".into(),
                    "create".into(),
                    "--private-key".into(),
                    "51ccv2".into(),
                    bucket.clone(),
                ],
                // Go checks "invalid hex character" but Rust secp256k1 produces a different message.
                expected_output: vec!["Invalid private key".into()],
                expect_error: true,
            },
        ];

        run_cases(cases).await;
    }

    // ── TestExternalDeleteBucketCommand ──────────────────────────────────────────

    #[tokio::test]
    async fn test_external_delete_bucket_command() {
        let pk = private_key();
        let first = random_name("tdb1");
        let second = random_name("tdb2");

        // Setup: create both buckets
        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &first,
        ])
        .await;
        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            "--metadata-encryption",
            "--encryption-key",
            ENC_KEY,
            &second,
        ])
        .await;

        let cases = vec![
            TestCase {
                name: "Delete non encrypted bucket with encryption",
                args: vec![
                    "bucket".into(),
                    "delete".into(),
                    "--private-key".into(),
                    pk.clone(),
                    first.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "--metadata-encryption".into(),
                    "--encryption-key".into(),
                    ENC_KEY.into(),
                ],
                expected_output: vec!["BucketNotFound".into()],
                expect_error: true,
            },
            TestCase {
                name: "Delete non encrypted bucket without encryption",
                args: vec![
                    "bucket".into(),
                    "delete".into(),
                    "--private-key".into(),
                    pk.clone(),
                    first.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec![format!("Bucket deleted: Name={first}")],
                expect_error: false,
            },
            TestCase {
                name: "Delete encrypted bucket without encryption",
                args: vec![
                    "bucket".into(),
                    "delete".into(),
                    "--private-key".into(),
                    pk.clone(),
                    second.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["BucketNotFound".into()],
                expect_error: true,
            },
            TestCase {
                name: "Delete encrypted bucket with encryption",
                args: vec![
                    "bucket".into(),
                    "delete".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "--metadata-encryption".into(),
                    "--encryption-key".into(),
                    ENC_KEY.into(),
                    second.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec![format!("Bucket deleted: Name={second}")],
                expect_error: false,
            },
            TestCase {
                name: "Delete not existing bucket",
                args: vec![
                    "bucket".into(),
                    "delete".into(),
                    "--private-key".into(),
                    pk.clone(),
                    first.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["BucketNotFound".into()],
                expect_error: true,
            },
            TestCase {
                name: "Empty bucket name provided",
                args: vec![
                    "bucket".into(),
                    "delete".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "".into(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["bucket name is required".into()],
                expect_error: true,
            },
        ];

        run_cases(cases).await;
    }

    // ── TestExternalViewBucketCommand ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_external_view_bucket_command() {
        let pk = private_key();
        let first = random_name("tvb1");
        let second = random_name("tvb2");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &first,
        ])
        .await;
        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            "--metadata-encryption",
            "--encryption-key",
            ENC_KEY,
            &second,
        ])
        .await;

        let cases = vec![
            TestCase {
                name: "View non encrypted bucket without encryption",
                args: vec![
                    "bucket".into(),
                    "view".into(),
                    "--private-key".into(),
                    pk.clone(),
                    first.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec![format!("Name={first}")],
                expect_error: false,
            },
            TestCase {
                name: "View non encrypted bucket with encryption",
                args: vec![
                    "bucket".into(),
                    "view".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "--metadata-encryption".into(),
                    "--encryption-key".into(),
                    ENC_KEY.into(),
                    first.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["BucketNotFound".into()],
                expect_error: true,
            },
            TestCase {
                name: "View encrypted bucket without encryption",
                args: vec![
                    "bucket".into(),
                    "view".into(),
                    "--private-key".into(),
                    pk.clone(),
                    second.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["BucketNotFound".into()],
                expect_error: true,
            },
            TestCase {
                name: "View encrypted bucket with encryption",
                args: vec![
                    "bucket".into(),
                    "view".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "--metadata-encryption".into(),
                    "--encryption-key".into(),
                    ENC_KEY.into(),
                    second.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec![format!("Name={second}")],
                expect_error: false,
            },
            TestCase {
                name: "View non-existent bucket",
                args: vec![
                    "bucket".into(),
                    "view".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "nonexistent-bucket-xyz".into(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["BucketNotFound".into()],
                expect_error: true,
            },
            TestCase {
                name: "Bucket name not provided",
                args: vec![
                    "bucket".into(),
                    "view".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "".into(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["bucket name is required".into()],
                expect_error: true,
            },
        ];

        run_cases(cases).await;
    }

    // ── TestExternalListBucketsCommand ────────────────────────────────────────────

    #[tokio::test]
    async fn test_external_list_buckets_command() {
        let pk = private_key();
        let names: Vec<String> = (0..3).map(|i| random_name(&format!("tlb{i}"))).collect();

        for n in &names {
            run_from_args(&[
                "--node-address",
                NODE_ADDRESS,
                "bucket",
                "create",
                "--private-key",
                &pk,
                n,
            ])
            .await;
        }

        // List buckets contains all test buckets
        {
            let (stdout, stderr, success) = run_from_args(&[
                "bucket",
                "list",
                "--node-address",
                NODE_ADDRESS,
                "--private-key",
                &pk,
                "--limit",
                "200",
            ])
            .await;
            let output = stdout + &stderr;
            assert!(success, "list buckets failed: {output}");
            for n in &names {
                assert!(
                    output.contains(&format!("Bucket: Name={n}")),
                    "missing {n} in: {output}"
                );
            }
        }

        // List buckets with limit=2
        {
            let (stdout, stderr, success) = run_from_args(&[
                "bucket",
                "list",
                "--node-address",
                NODE_ADDRESS,
                "--private-key",
                &pk,
                "--offset",
                "0",
                "--limit",
                "2",
            ])
            .await;
            let output = stdout + &stderr;
            assert!(success, "list with limit failed: {output}");
            let count = output
                .lines()
                .filter(|l| l.contains("Bucket: Name="))
                .count();
            assert!(count <= 2, "expected at most 2, got {count} in: {output}");
        }

        // Pagination returns all test buckets
        {
            let mut found: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut offset = 0i64;
            let limit = 2i64;
            loop {
                let off = offset.to_string();
                let lim = limit.to_string();
                let (stdout, stderr, success) = run_from_args(&[
                    "bucket",
                    "list",
                    "--node-address",
                    NODE_ADDRESS,
                    "--private-key",
                    &pk,
                    "--offset",
                    &off,
                    "--limit",
                    &lim,
                ])
                .await;
                assert!(success);
                let output = stdout + &stderr;
                let before = found.len();
                for line in output.lines() {
                    if let Some(rest) = line.strip_prefix("Bucket: Name=") {
                        let name = rest.split(',').next().unwrap_or(rest).to_string();
                        found.insert(name);
                    }
                }
                if found.len() == before {
                    break;
                }
                offset += limit;
            }
            for n in &names {
                assert!(
                    found.contains(n.as_str()),
                    "missing {n} in paginated results"
                );
            }
        }
    }

    // ── TestExternalListBucketsCommandWithMetdadataEncryption ─────────────────────

    #[tokio::test]
    async fn test_external_list_buckets_command_with_metadata_encryption() {
        let pk = private_key();
        let names: Vec<String> = (0..3).map(|i| random_name(&format!("tlbenc{i}"))).collect();

        for n in &names {
            run_from_args(&[
                "--node-address",
                NODE_ADDRESS,
                "bucket",
                "create",
                "--private-key",
                &pk,
                "--metadata-encryption",
                "--encryption-key",
                ENC_KEY,
                n,
            ])
            .await;
        }

        // List buckets contains all test encrypted buckets
        {
            let (stdout, stderr, success) = run_from_args(&[
                "bucket",
                "list",
                "--node-address",
                NODE_ADDRESS,
                "--private-key",
                &pk,
                "--metadata-encryption",
                "--encryption-key",
                ENC_KEY,
                "--limit",
                "200",
            ])
            .await;
            let output = stdout + &stderr;
            assert!(success, "list encrypted buckets failed: {output}");
            for n in &names {
                assert!(
                    output.contains(&format!("Bucket: Name={n}")),
                    "missing {n} in: {output}"
                );
            }
        }

        // List buckets with limit=2 returns exactly 2
        {
            let (stdout, stderr, success) = run_from_args(&[
                "bucket",
                "list",
                "--node-address",
                NODE_ADDRESS,
                "--private-key",
                &pk,
                "--metadata-encryption",
                "--encryption-key",
                ENC_KEY,
                "--offset",
                "0",
                "--limit",
                "2",
            ])
            .await;
            let output = stdout + &stderr;
            assert!(success, "list enc with limit failed: {output}");
            let count = output
                .lines()
                .filter(|l| l.contains("Bucket: Name="))
                .count();
            assert!(count <= 2, "expected at most 2, got {count} in: {output}");
        }

        // Pagination
        {
            let mut found: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut offset = 0i64;
            let limit = 2i64;
            loop {
                let off = offset.to_string();
                let lim = limit.to_string();
                let (stdout, stderr, success) = run_from_args(&[
                    "bucket",
                    "list",
                    "--node-address",
                    NODE_ADDRESS,
                    "--private-key",
                    &pk,
                    "--metadata-encryption",
                    "--encryption-key",
                    ENC_KEY,
                    "--offset",
                    &off,
                    "--limit",
                    &lim,
                ])
                .await;
                assert!(success);
                let output = stdout + &stderr;
                let before = found.len();
                for line in output.lines() {
                    if let Some(rest) = line.strip_prefix("Bucket: Name=") {
                        let name = rest.split(',').next().unwrap_or(rest).to_string();
                        found.insert(name);
                    }
                }
                if found.len() == before {
                    break;
                }
                offset += limit;
            }
            for n in &names {
                assert!(
                    found.contains(n.as_str()),
                    "missing {n} in paginated enc results"
                );
            }
        }
    }

    // ── TestExternalListFilesCommand ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_external_list_files_command() {
        let pk = private_key();
        let bucket = random_name("tlf");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &bucket,
        ])
        .await;

        let mut file_names = Vec::new();
        for _ in 0..3 {
            let (file_path, file_name) = create_temp_file(FILE_SIZE);
            run_from_args(&[
                "file",
                "upload",
                "--private-key",
                &pk,
                &bucket,
                &file_path,
                "--node-address",
                NODE_ADDRESS,
            ])
            .await;
            file_names.push(file_name);
        }

        let cases = vec![
            TestCase {
                name: "List files successfully",
                args: vec![
                    "file".into(),
                    "list".into(),
                    bucket.clone(),
                    "--private-key".into(),
                    pk.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "--limit".into(),
                    "200".into(),
                ],
                expected_output: {
                    let mut v: Vec<String> = file_names
                        .iter()
                        .map(|n| format!("File: Name={n}"))
                        .collect();
                    v.push(format!("ActualSize={FILE_SIZE}"));
                    v
                },
                expect_error: false,
            },
            TestCase {
                name: "List files with limit=2",
                args: vec![
                    "file".into(),
                    "list".into(),
                    bucket.clone(),
                    "--private-key".into(),
                    pk.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "--offset".into(),
                    "0".into(),
                    "--limit".into(),
                    "2".into(),
                ],
                expected_output: vec![
                    format!("File: Name={}", file_names[0]),
                    format!("File: Name={}", file_names[1]),
                ],
                expect_error: false,
            },
            TestCase {
                name: "List files with offset=1 and limit=2",
                args: vec![
                    "file".into(),
                    "list".into(),
                    bucket.clone(),
                    "--private-key".into(),
                    pk.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "--offset".into(),
                    "1".into(),
                    "--limit".into(),
                    "2".into(),
                ],
                expected_output: vec![
                    format!("File: Name={}", file_names[1]),
                    format!("File: Name={}", file_names[2]),
                ],
                expect_error: false,
            },
            TestCase {
                name: "List files with offset beyond range",
                args: vec![
                    "file".into(),
                    "list".into(),
                    bucket.clone(),
                    "--private-key".into(),
                    pk.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "--offset".into(),
                    "10".into(),
                    "--limit".into(),
                    "2".into(),
                ],
                expected_output: vec!["No files".into()],
                expect_error: false,
            },
            TestCase {
                name: "List files for non-existent bucket",
                args: vec![
                    "file".into(),
                    "list".into(),
                    "nonexistent-bucket-xyz".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["No files".into()],
                expect_error: false,
            },
            TestCase {
                name: "Bucket name not provided",
                args: vec![
                    "file".into(),
                    "list".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "".into(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["bucket name is required".into()],
                expect_error: true,
            },
        ];

        run_cases(cases).await;

        // Pagination: pages should be non-overlapping and cover all files
        {
            let off1 = "0";
            let lim = "2";
            let (s1, e1, ok1) = run_from_args(&[
                "file",
                "list",
                &bucket,
                "--private-key",
                &pk,
                "--node-address",
                NODE_ADDRESS,
                "--offset",
                off1,
                "--limit",
                lim,
            ])
            .await;
            assert!(ok1);
            let out1 = s1 + &e1;

            let off2 = "2";
            let (s2, e2, ok2) = run_from_args(&[
                "file",
                "list",
                &bucket,
                "--private-key",
                &pk,
                "--node-address",
                NODE_ADDRESS,
                "--offset",
                off2,
                "--limit",
                lim,
            ])
            .await;
            assert!(ok2);
            let out2 = s2 + &e2;

            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for n in &file_names {
                let in1 = out1.contains(n.as_str());
                let in2 = out2.contains(n.as_str());
                assert!(!(in1 && in2), "file {n} appears in both pages (duplicate)");
                seen.insert(n.clone());
            }
            for n in &file_names {
                let found = out1.contains(n.as_str()) || out2.contains(n.as_str());
                assert!(found, "file {n} not found in any page");
            }
        }
    }

    // ── TestExternalListFilesCommandWithMetadataEncryption ────────────────────────

    #[tokio::test]
    async fn test_external_list_files_command_with_metadata_encryption() {
        let pk = private_key();
        let bucket = random_name("tlfenc");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            "--metadata-encryption",
            "--encryption-key",
            ENC_KEY,
            &bucket,
        ])
        .await;

        let mut file_names = Vec::new();
        for _ in 0..3 {
            let (file_path, file_name) = create_temp_file(FILE_SIZE);
            run_from_args(&[
                "file",
                "upload",
                "--private-key",
                &pk,
                "--metadata-encryption",
                "--encryption-key",
                ENC_KEY,
                &bucket,
                &file_path,
                "--node-address",
                NODE_ADDRESS,
            ])
            .await;
            file_names.push(file_name);
        }

        // Without metadata encryption flag → no files visible
        {
            let (stdout, stderr, success) = run_from_args(&[
                "file",
                "list",
                &bucket,
                "--private-key",
                &pk,
                "--node-address",
                NODE_ADDRESS,
            ])
            .await;
            let out = stdout + &stderr;
            assert!(success, "list without enc failed: {out}");
            assert!(
                out.contains("No files"),
                "expected 'No files' without enc flag: {out}"
            );
        }

        // With metadata encryption flag → all files visible with ActualSize
        {
            let (stdout, stderr, success) = run_from_args(&[
                "file",
                "list",
                &bucket,
                "--private-key",
                &pk,
                "--metadata-encryption",
                "--encryption-key",
                ENC_KEY,
                "--node-address",
                NODE_ADDRESS,
                "--limit",
                "200",
            ])
            .await;
            let out = stdout + &stderr;
            assert!(success, "list with enc failed: {out}");
            for n in &file_names {
                assert!(
                    out.contains(&format!("File: Name={n}")),
                    "missing {n}: {out}"
                );
            }
            assert!(
                out.contains(&format!("ActualSize={FILE_SIZE}")),
                "missing ActualSize: {out}"
            );
        }
    }

    // ── TestExternalFileInfoCommand ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_external_file_info_command() {
        let pk = private_key();
        let bucket = random_name("tfi");
        let enc_bucket = random_name("tfi-enc");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &bucket,
        ])
        .await;
        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            "--metadata-encryption",
            "--encryption-key",
            ENC_KEY,
            &enc_bucket,
        ])
        .await;

        let (file_path, file_name) = create_temp_file(FILE_SIZE);
        run_from_args(&[
            "file",
            "upload",
            "--private-key",
            &pk,
            &bucket,
            &file_path,
            "--node-address",
            NODE_ADDRESS,
        ])
        .await;

        let (file2_path, file2_name) = create_temp_file(FILE_SIZE);
        run_from_args(&[
            "file",
            "upload",
            "--private-key",
            &pk,
            "--metadata-encryption",
            "--encryption-key",
            ENC_KEY,
            &enc_bucket,
            &file2_path,
            "--node-address",
            NODE_ADDRESS,
        ])
        .await;

        let cases = vec![
            TestCase {
                name: "File info successfully",
                args: vec![
                    "file".into(),
                    "info".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    file_name.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec![
                    format!("File: Name={file_name}"),
                    "RootCID=".into(),
                    format!("ActualSize={FILE_SIZE}"),
                ],
                expect_error: false,
            },
            TestCase {
                name: "File info for encrypted bucket without metadata encryption flag",
                args: vec![
                    "file".into(),
                    "info".into(),
                    "--private-key".into(),
                    pk.clone(),
                    enc_bucket.clone(),
                    file2_name.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["BucketNotFound".into()],
                expect_error: true,
            },
            TestCase {
                name: "File info for non-encrypted bucket with metadata encryption flag",
                args: vec![
                    "file".into(),
                    "info".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "--metadata-encryption".into(),
                    "--encryption-key".into(),
                    ENC_KEY.into(),
                    bucket.clone(),
                    file_name.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["BucketNotFound".into()],
                expect_error: true,
            },
            TestCase {
                name: "File info for non-existent file",
                args: vec![
                    "file".into(),
                    "info".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    "nonexistent-file".into(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["file not exists".into()],
                expect_error: true,
            },
            TestCase {
                name: "Bucket name not provided",
                args: vec![
                    "file".into(),
                    "info".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "".into(),
                    file_name.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["bucket name is required".into()],
                expect_error: true,
            },
            TestCase {
                name: "File name not provided",
                args: vec![
                    "file".into(),
                    "info".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    "".into(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["file name is required".into()],
                expect_error: true,
            },
        ];

        run_cases(cases).await;
    }

    // ── TestExternalFileUploadCommand ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_external_file_upload_command() {
        let pk = private_key();
        let bucket = random_name("tfu");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &bucket,
        ])
        .await;

        let (file_path, file_name) = create_temp_file(FILE_SIZE);
        let (file2_path, file2_name) = create_temp_file(FILE_SIZE);

        let cases = vec![
            TestCase {
                name: "File upload successfully without erasure coding",
                args: vec![
                    "file".into(),
                    "upload".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    file_path.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "--disable-erasure-coding".into(),
                ],
                expected_output: vec![
                    format!("File uploaded successfully: Name={file_name}"),
                    format!("ActualSize={FILE_SIZE}"),
                ],
                expect_error: false,
            },
            TestCase {
                name: "File upload successfully with erasure coding",
                args: vec![
                    "file".into(),
                    "upload".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    file2_path.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec![format!("File uploaded successfully: Name={file2_name}")],
                expect_error: false,
            },
            TestCase {
                name: "Bucket name not provided",
                args: vec![
                    "file".into(),
                    "upload".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "".into(),
                    file_path.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["bucket name is required".into()],
                expect_error: true,
            },
            TestCase {
                name: "File path not provided",
                args: vec![
                    "file".into(),
                    "upload".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    "".into(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["file path is required".into()],
                expect_error: true,
            },
        ];

        run_cases(cases).await;
    }

    // ── TestExternalFileUploadCommandWithMetadataEncryption ───────────────────────

    #[tokio::test]
    async fn test_external_file_upload_command_with_metadata_encryption() {
        let pk = private_key();
        let bucket = random_name("tfu-menc");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            "--metadata-encryption",
            "--encryption-key",
            ENC_KEY,
            &bucket,
        ])
        .await;

        let (file_path, file_name) = create_temp_file(FILE_SIZE);

        run_cases(vec![TestCase {
            name: "File upload successfully with metadata encryption",
            args: vec![
                "file".into(),
                "upload".into(),
                "--private-key".into(),
                pk.clone(),
                "--metadata-encryption".into(),
                "--encryption-key".into(),
                ENC_KEY.into(),
                bucket.clone(),
                file_path.clone(),
                "--node-address".into(),
                NODE_ADDRESS.into(),
            ],
            expected_output: vec![format!("File uploaded successfully: Name={file_name}")],
            expect_error: false,
        }])
        .await;
    }

    // ── TestExternalFileUploadCommandWithEncryption ───────────────────────────────

    #[tokio::test]
    async fn test_external_file_upload_command_with_encryption() {
        let pk = private_key();
        let bucket = random_name("tfu-enc");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &bucket,
        ])
        .await;

        let (file_path, file_name) = create_temp_file(FILE_SIZE);
        let (file2_path, file2_name) = create_temp_file(FILE_SIZE);

        run_cases(vec![
            TestCase {
                name: "File upload successfully with encryption without erasure coding",
                args: vec![
                    "file".into(),
                    "upload".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    file_path.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "--encryption-key".into(),
                    ENC_KEY.into(),
                    "--disable-erasure-coding".into(),
                ],
                expected_output: vec![format!("File uploaded successfully: Name={file_name}")],
                expect_error: false,
            },
            TestCase {
                name: "File upload successfully with encryption and erasure coding",
                args: vec![
                    "file".into(),
                    "upload".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    file2_path.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "--encryption-key".into(),
                    ENC_KEY.into(),
                ],
                expected_output: vec![format!("File uploaded successfully: Name={file2_name}")],
                expect_error: false,
            },
        ])
        .await;
    }

    // ── TestExternalFileDownloadCommand ───────────────────────────────────────────

    #[tokio::test]
    async fn test_external_file_download_command() {
        let pk = private_key();
        let bucket = random_name("tfdl");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &bucket,
        ])
        .await;

        let (file_path, file_name) = create_temp_file(FILE_SIZE);
        run_from_args(&[
            "file",
            "upload",
            "--private-key",
            &pk,
            &bucket,
            &file_path,
            "--node-address",
            NODE_ADDRESS,
        ])
        .await;

        let tmp = create_temp_dir();

        run_cases(vec![
            TestCase {
                name: "File download successfully",
                args: vec![
                    "file".into(),
                    "download".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    file_name.clone(),
                    tmp.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec![format!("File downloaded successfully: Name={file_name}")],
                expect_error: false,
            },
            TestCase {
                name: "Bucket name not provided",
                args: vec![
                    "file".into(),
                    "download".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "".into(),
                    file_name.clone(),
                    tmp.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["bucket name is required".into()],
                expect_error: true,
            },
            TestCase {
                name: "File name not provided",
                args: vec![
                    "file".into(),
                    "download".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    "".into(),
                    tmp.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["file name is required".into()],
                expect_error: true,
            },
            TestCase {
                name: "Destination path not provided",
                args: vec![
                    "file".into(),
                    "download".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    file_name.clone(),
                    "".into(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["destination path is required".into()],
                expect_error: true,
            },
        ])
        .await;
    }

    // ── TestExternalFileDownloadCommandWithMetadataEncryption ─────────────────────

    #[tokio::test]
    async fn test_external_file_download_command_with_metadata_encryption() {
        let pk = private_key();
        let bucket = random_name("tfdl-menc");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            "--metadata-encryption",
            "--encryption-key",
            ENC_KEY,
            &bucket,
        ])
        .await;

        let (file_path, file_name) = create_temp_file(FILE_SIZE);
        run_from_args(&[
            "file",
            "upload",
            "--private-key",
            &pk,
            "--metadata-encryption",
            "--encryption-key",
            ENC_KEY,
            &bucket,
            &file_path,
            "--node-address",
            NODE_ADDRESS,
        ])
        .await;

        let tmp = create_temp_dir();

        run_cases(vec![TestCase {
            name: "File download successfully with metadata encryption",
            args: vec![
                "file".into(),
                "download".into(),
                "--private-key".into(),
                pk.clone(),
                "--metadata-encryption".into(),
                "--encryption-key".into(),
                ENC_KEY.into(),
                bucket.clone(),
                file_name.clone(),
                tmp.clone(),
                "--node-address".into(),
                NODE_ADDRESS.into(),
            ],
            expected_output: vec![format!("File downloaded successfully: Name={file_name}")],
            expect_error: false,
        }])
        .await;
    }

    // ── TestExternalFileDownloadCommandWithErasureCoding ──────────────────────────

    #[tokio::test]
    async fn test_external_file_download_command_with_erasure_coding() {
        let pk = private_key();
        let bucket = random_name("tfdl-ec");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &bucket,
        ])
        .await;

        let (file_path, file_name) = create_temp_file(FILE_SIZE);
        run_from_args(&[
            "file",
            "upload",
            "--private-key",
            &pk,
            &bucket,
            &file_path,
            "--node-address",
            NODE_ADDRESS,
        ])
        .await;

        let tmp = create_temp_dir();

        run_cases(vec![TestCase {
            name: "File download successfully with erasure coding",
            args: vec![
                "file".into(),
                "download".into(),
                "--private-key".into(),
                pk.clone(),
                bucket.clone(),
                file_name.clone(),
                tmp.clone(),
                "--node-address".into(),
                NODE_ADDRESS.into(),
            ],
            expected_output: vec![format!("File downloaded successfully: Name={file_name}")],
            expect_error: false,
        }])
        .await;
    }

    // ── TestExternalFileDownloadCommandWithEncryption ─────────────────────────────
    // Mirrors Go's test: file uploaded with enc key, downloaded with empty key
    // (gets encrypted bytes without decryption — no error).

    #[tokio::test]
    async fn test_external_file_download_command_with_encryption() {
        let pk = private_key();
        let bucket = random_name("tfdl-enc");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &bucket,
        ])
        .await;

        let (file_path, file_name) = create_temp_file(FILE_SIZE);
        run_from_args(&[
            "file",
            "upload",
            "--private-key",
            &pk,
            &bucket,
            &file_path,
            "--node-address",
            NODE_ADDRESS,
            "--encryption-key",
            ENC_KEY,
        ])
        .await;

        let tmp = create_temp_dir();

        run_cases(vec![TestCase {
            name: "File download successfully without decryption key",
            args: vec![
                "file".into(),
                "download".into(),
                "--private-key".into(),
                pk.clone(),
                bucket.clone(),
                file_name.clone(),
                tmp.clone(),
                "--node-address".into(),
                NODE_ADDRESS.into(),
                "-e".into(),
                "".into(),
            ],
            expected_output: vec![format!("File downloaded successfully: Name={file_name}")],
            expect_error: false,
        }])
        .await;
    }

    // ── TestExternalFileDownloadCommandWithEncryptionAndErasureCoding ─────────────

    #[tokio::test]
    async fn test_external_file_download_command_with_encryption_and_erasure_coding() {
        let pk = private_key();
        let bucket = random_name("tfdl-ec-enc");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &bucket,
        ])
        .await;

        let (file_path, file_name) = create_temp_file(FILE_SIZE);
        run_from_args(&[
            "file",
            "upload",
            "--private-key",
            &pk,
            &bucket,
            &file_path,
            "--node-address",
            NODE_ADDRESS,
            "--encryption-key",
            ENC_KEY,
        ])
        .await;

        let tmp = create_temp_dir();

        run_cases(vec![TestCase {
            name: "File download successfully with encryption and erasure coding",
            args: vec![
                "file".into(),
                "download".into(),
                "--private-key".into(),
                pk.clone(),
                bucket.clone(),
                file_name.clone(),
                tmp.clone(),
                "--node-address".into(),
                NODE_ADDRESS.into(),
                "--encryption-key".into(),
                ENC_KEY.into(),
            ],
            expected_output: vec![format!("File downloaded successfully: Name={file_name}")],
            expect_error: false,
        }])
        .await;
    }

    // ── TestExternalFileDeleteCommand ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_external_file_delete_command() {
        let pk = private_key();
        let bucket = random_name("tfde");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &bucket,
        ])
        .await;

        let (file_path, file_name) = create_temp_file(FILE_SIZE);
        run_from_args(&[
            "file",
            "upload",
            "--private-key",
            &pk,
            &bucket,
            &file_path,
            "--node-address",
            NODE_ADDRESS,
        ])
        .await;

        run_cases(vec![
            TestCase {
                name: "File delete successfully",
                args: vec![
                    "file".into(),
                    "delete".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    file_name.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec![format!("File successfully deleted: Name={file_name}")],
                expect_error: false,
            },
            TestCase {
                name: "Bucket name not provided",
                args: vec![
                    "file".into(),
                    "delete".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "".into(),
                    file_name.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["bucket name is required".into()],
                expect_error: true,
            },
            TestCase {
                name: "File name not provided",
                args: vec![
                    "file".into(),
                    "delete".into(),
                    "--private-key".into(),
                    pk.clone(),
                    bucket.clone(),
                    "".into(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["file name is required".into()],
                expect_error: true,
            },
        ])
        .await;
    }

    // ── TestExternalFileDeleteCommandWithMetadataEncryption ───────────────────────

    #[tokio::test]
    async fn test_external_file_delete_command_with_metadata_encryption() {
        let pk = private_key();
        let bucket = random_name("tfde-enc");

        run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            "--metadata-encryption",
            "--encryption-key",
            ENC_KEY,
            &bucket,
        ])
        .await;

        let (file_path, file_name) = create_temp_file(FILE_SIZE);
        run_from_args(&[
            "file",
            "upload",
            "--private-key",
            &pk,
            "--metadata-encryption",
            "--encryption-key",
            ENC_KEY,
            &bucket,
            &file_path,
            "--node-address",
            NODE_ADDRESS,
        ])
        .await;

        run_cases(vec![TestCase {
            name: "File delete successfully with metadata encryption",
            args: vec![
                "file".into(),
                "delete".into(),
                "--private-key".into(),
                pk.clone(),
                "--metadata-encryption".into(),
                "--encryption-key".into(),
                ENC_KEY.into(),
                bucket.clone(),
                file_name.clone(),
                "--node-address".into(),
                NODE_ADDRESS.into(),
            ],
            expected_output: vec![format!("File successfully deleted: Name={file_name}")],
            expect_error: false,
        }])
        .await;
    }

    // ── TestExternalWalletBalanceCommand ──────────────────────────────────────

    #[tokio::test]
    async fn test_external_wallet_balance_command() {
        const TEST_PK: &str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let wallet_name = random_name("twbc");
        let keystore = create_temp_dir();

        // Import the well-known Anvil funded account into the keystore.
        let (stdout, stderr, ok) = run_from_args(&[
            "wallet",
            "import",
            &wallet_name,
            TEST_PK,
            "--keystore",
            &keystore,
        ])
        .await;
        assert!(ok, "wallet import failed: {}", stdout + &stderr);
        assert!(
            (stdout + &stderr).contains("Wallet imported successfully"),
            "unexpected import output"
        );

        run_cases(vec![
            TestCase {
                name: "Get wallet balance successfully",
                args: vec![
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "wallet".into(),
                    "balance".into(),
                    wallet_name.clone(),
                    "--keystore".into(),
                    keystore.clone(),
                ],
                expected_output: vec![
                    "Wallet:".into(),
                    wallet_name.clone(),
                    "Address:".into(),
                    "Balance:".into(),
                    "AKVT".into(),
                ],
                expect_error: false,
            },
            TestCase {
                name: "Get balance for default wallet (no wallet name specified)",
                args: vec![
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "wallet".into(),
                    "balance".into(),
                    "--keystore".into(),
                    keystore.clone(),
                ],
                expected_output: vec![
                    "Wallet:".into(),
                    wallet_name.clone(),
                    "Address:".into(),
                    "Balance:".into(),
                    "AKVT".into(),
                ],
                expect_error: false,
            },
            TestCase {
                name: "Get balance for non-existent wallet",
                args: vec![
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                    "wallet".into(),
                    "balance".into(),
                    "non-existent-wallet".into(),
                    "--keystore".into(),
                    keystore.clone(),
                ],
                expected_output: vec!["failed to read wallet file".into()],
                expect_error: true,
            },
        ])
        .await;
    }

    // ── TestPDPExternalArchivalMetadataCommand ────────────────────────────────

    #[tokio::test]
    async fn test_external_pdp_archival_metadata_command() {
        let pk = private_key();
        let bucket = random_name("tpam");

        let (_, stderr, ok) = run_from_args(&[
            "--node-address",
            NODE_ADDRESS,
            "bucket",
            "create",
            "--private-key",
            &pk,
            &bucket,
        ])
        .await;
        assert!(ok, "setup: bucket create failed: {stderr}");

        let (file_path, file_name) = create_temp_file(FILE_SIZE);
        let (_, stderr, ok) = run_from_args(&[
            "file",
            "upload",
            "--private-key",
            &pk,
            &bucket,
            &file_path,
            "--node-address",
            NODE_ADDRESS,
        ])
        .await;
        assert!(ok, "setup: file upload failed: {stderr}");

        run_cases(vec![
            TestCase {
                name: "File not fully available",
                args: vec![
                    "file".into(),
                    "archival-metadata".into(),
                    bucket.clone(),
                    file_name.clone(),
                    "--private-key".into(),
                    pk.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["Not fully available in archival storage".into()],
                expect_error: false,
            },
            TestCase {
                name: "Verbose mode",
                args: vec![
                    "file".into(),
                    "archival-metadata".into(),
                    bucket.clone(),
                    file_name.clone(),
                    "--private-key".into(),
                    pk.clone(),
                    "-v".into(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec![
                    "Total Chunks:".into(),
                    "Chunk CID:".into(),
                    "Block CID:".into(),
                ],
                expect_error: false,
            },
            TestCase {
                name: "File not found",
                args: vec![
                    "file".into(),
                    "archival-metadata".into(),
                    bucket.clone(),
                    "nonexistent-file".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                // Server returns gRPC message "FileDoesNotExist" (Go SDK maps this to "file not exists")
                expected_output: vec!["FileDoesNotExist".into()],
                expect_error: true,
            },
            TestCase {
                name: "Bucket name not provided",
                args: vec![
                    "file".into(),
                    "archival-metadata".into(),
                    "".into(),
                    file_name.clone(),
                    "--private-key".into(),
                    pk.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["bucket name is required".into()],
                expect_error: true,
            },
            TestCase {
                name: "File name not provided",
                args: vec![
                    "file".into(),
                    "archival-metadata".into(),
                    bucket.clone(),
                    "".into(),
                    "--private-key".into(),
                    pk.clone(),
                    "--node-address".into(),
                    NODE_ADDRESS.into(),
                ],
                expected_output: vec!["file name is required".into()],
                expect_error: true,
            },
        ])
        .await;
    }

    // ── TestWalletFlow ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_wallet_flow() {
        let keystore = std::env::temp_dir()
            .join(format!("akave-wallet-test-{}", random_name("ks")))
            .to_string_lossy()
            .into_owned();
        let wallet_name = random_name("wallet");

        let (stdout, stderr, ok) =
            run_from_args(&["wallet", "create", &wallet_name, "--keystore", &keystore]).await;
        let combined = stdout + &stderr;
        assert!(ok, "wallet create failed: {combined}");
        assert!(
            combined.contains(&format!("Wallet ({wallet_name}) created successfully")),
            "unexpected output: {combined}"
        );

        let (stdout, stderr, ok) =
            run_from_args(&["wallet", "list", "--keystore", &keystore]).await;
        let combined = stdout + &stderr;
        assert!(ok, "wallet list failed: {combined}");
        assert!(
            combined.contains(&wallet_name),
            "name not in list: {combined}"
        );

        let (stdout, stderr, ok) = run_from_args(&[
            "wallet",
            "export-key",
            &wallet_name,
            "--keystore",
            &keystore,
        ])
        .await;
        let combined = stdout + &stderr;
        assert!(ok, "wallet export-key failed: {combined}");
        assert!(
            combined.contains("Private key:"),
            "no private key in output: {combined}"
        );

        let _ = std::fs::remove_dir_all(&keystore);
    }
}
