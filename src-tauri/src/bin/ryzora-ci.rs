use ryzora_lib::crypto::{
    compute_key_fingerprint, evaluate_package_directory_crypto, generate_ed25519_keypair,
    load_author_signing_key, sign_package_tree_hash, write_private_key_atomic, SignerIdentity,
    TrustStore, VerifyingKey,
};
use ryzora_lib::ingestion::{ingest_submission_into_repository, run_ci_audit};
use ryzora_lib::installer::load_package_manifest;
use ryzora_lib::repository::compute_package_tree_hash;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn print_usage() {
    eprintln!("Ryzora Release & CI Tool (ryzora-ci)");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  ryzora-ci audit <submission-dir> [--repo <repo-dir>] [--markdown-out <file>]");
    eprintln!("  ryzora-ci ingest <submission-dir> <repo-dir>");
    eprintln!("  ryzora-ci keygen <output-prefix>");
    eprintln!("  ryzora-ci fingerprint <pubkey-hex-or-file>");
    eprintln!("  ryzora-ci sign-package <pkg-dir> <private-key-file> [--author-id <id>] [--author-name <name>] [--author-handle <handle>]");
    eprintln!("  ryzora-ci verify-package <pkg-dir>");
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return ExitCode::from(1);
    }

    let command = &args[1];
    match command.as_str() {
        "--help" | "-h" | "help" => {
            print_usage();
            ExitCode::SUCCESS
        }
        "audit" => {
            if args.len() < 3 {
                print_usage();
                return ExitCode::from(1);
            }
            let submission_dir = PathBuf::from(&args[2]);
            let mut repo_dir: Option<PathBuf> = None;
            let mut markdown_out: Option<PathBuf> = None;

            let mut i = 3;
            while i < args.len() {
                if args[i] == "--repo" && i + 1 < args.len() {
                    repo_dir = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else if args[i] == "--markdown-out" && i + 1 < args.len() {
                    markdown_out = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("Unknown argument: {}", args[i]);
                    print_usage();
                    return ExitCode::from(1);
                }
            }

            let trust_store = TrustStore::load_default();

            match run_ci_audit(&submission_dir, repo_dir.as_deref(), &trust_store) {
                Ok(report) => {
                    println!("==================================================");
                    println!(
                        "Ryzora CI Submission Audit: {}",
                        if report.passed { "PASSED" } else { "FAILED" }
                    );
                    println!("Package: {} v{}", report.package_id, report.version);
                    println!("Audit Score: {}/100", report.audit_score);
                    println!("Computed Trust Tier: {:?}", report.computed_trust_tier);
                    println!("Moderation Status: {:?}", report.moderation_status);
                    println!("==================================================");
                    for check in &report.checks {
                        let status_icon = if check.passed { "[PASS]" } else { "[FAIL]" };
                        println!("{} {}: {}", status_icon, check.name, check.message);
                    }
                    if !report.errors.is_empty() {
                        eprintln!("\nErrors:");
                        for err in &report.errors {
                            eprintln!("  - {}", err);
                        }
                    }
                    if !report.warnings.is_empty() {
                        println!("\nWarnings:");
                        for warn in &report.warnings {
                            println!("  - {}", warn);
                        }
                    }

                    if let Some(md_path) = markdown_out {
                        if let Some(parent) = md_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if let Err(e) = std::fs::write(&md_path, &report.pr_comment_markdown) {
                            eprintln!("Failed to write markdown output to {:?}: {}", md_path, e);
                            return ExitCode::from(1);
                        }
                        println!("\nMarkdown comment generated: {:?}", md_path);
                    }

                    if report.passed {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(e) => {
                    eprintln!("CI audit error: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        "ingest" => {
            if args.len() < 4 {
                print_usage();
                return ExitCode::from(1);
            }
            let submission_dir = PathBuf::from(&args[2]);
            let repo_dir = PathBuf::from(&args[3]);

            let trust_store = TrustStore::load_default();

            match ingest_submission_into_repository(&submission_dir, &repo_dir, &trust_store) {
                Ok(result) => {
                    println!(
                        "Successfully ingested {} v{} into {:?}",
                        result.package_id, result.version, repo_dir
                    );
                    println!("Repository index updated at: {}", result.repository_path);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Ingestion failed (repository cleanly rolled back): {}", e);
                    ExitCode::from(1)
                }
            }
        }
        "keygen" => {
            if args.len() < 3 {
                eprintln!("Usage: ryzora-ci keygen <output-prefix>");
                return ExitCode::from(1);
            }
            let prefix = &args[2];
            let pub_path = PathBuf::from(format!("{}.pub", prefix));
            let key_path = PathBuf::from(format!("{}.key", prefix));
            let id_path = PathBuf::from(format!("{}.id", prefix));

            let (signing_key, verifying_key) = generate_ed25519_keypair();
            let pub_hex = hex::encode(verifying_key.as_bytes());
            let priv_hex = hex::encode(signing_key.to_bytes());
            let fingerprint = compute_key_fingerprint(&verifying_key);

            // Write public key
            if let Err(e) = fs::write(&pub_path, format!("{}\n", pub_hex)) {
                eprintln!("Failed to write public key to {:?}: {}", pub_path, e);
                return ExitCode::from(1);
            }

            // Write private key with strict 0600 permissions
            if let Err(e) = write_private_key_atomic(&key_path, &priv_hex) {
                eprintln!("Failed to write private key to {:?}: {}", key_path, e);
                return ExitCode::from(1);
            }

            // Write fingerprint ID
            if let Err(e) = fs::write(&id_path, format!("{}\n", fingerprint)) {
                eprintln!("Failed to write ID to {:?}: {}", id_path, e);
                return ExitCode::from(1);
            }

            println!("==================================================");
            println!("Ryzora Ed25519 Keypair Generated");
            println!("Fingerprint ID: {}", fingerprint);
            println!("Public Key:     {:?}", pub_path);
            println!("Private Key:    {:?} (mode 0600, keep secret!)", key_path);
            println!("==================================================");
            ExitCode::SUCCESS
        }
        "fingerprint" => {
            if args.len() < 3 {
                eprintln!("Usage: ryzora-ci fingerprint <pubkey-hex-or-file>");
                return ExitCode::from(1);
            }
            let input = &args[2];
            let hex_str = if Path::new(input).is_file() {
                match fs::read_to_string(input) {
                    Ok(s) => s.trim().to_string(),
                    Err(e) => {
                        eprintln!("Failed to read public key file '{}': {}", input, e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                input.trim().to_string()
            };

            let bytes = match hex::decode(&hex_str) {
                Ok(b) if b.len() == 32 => b,
                Ok(b) => {
                    eprintln!("Invalid key length: expected 32 bytes, got {}", b.len());
                    return ExitCode::from(1);
                }
                Err(e) => {
                    eprintln!("Invalid hex format: {}", e);
                    return ExitCode::from(1);
                }
            };

            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            match VerifyingKey::from_bytes(&arr) {
                Ok(vk) => {
                    let fp = compute_key_fingerprint(&vk);
                    println!("{}", fp);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Failed to parse Ed25519 public key: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        "sign-package" => {
            if args.len() < 4 {
                eprintln!("Usage: ryzora-ci sign-package <pkg-dir> <private-key-file> [--author-id <id>] [--author-name <name>] [--author-handle <handle>]");
                return ExitCode::from(1);
            }
            let pkg_dir = PathBuf::from(&args[2]);
            let key_path = PathBuf::from(&args[3]);

            let mut author_id: Option<String> = None;
            let mut author_name: Option<String> = None;
            let mut author_handle: Option<String> = None;

            let mut i = 4;
            while i < args.len() {
                if args[i] == "--author-id" && i + 1 < args.len() {
                    author_id = Some(args[i + 1].clone());
                    i += 2;
                } else if args[i] == "--author-name" && i + 1 < args.len() {
                    author_name = Some(args[i + 1].clone());
                    i += 2;
                } else if args[i] == "--author-handle" && i + 1 < args.len() {
                    author_handle = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Unknown argument: {}", args[i]);
                    print_usage();
                    return ExitCode::from(1);
                }
            }

            // 1. Load manifest
            let manifest = match load_package_manifest(&pkg_dir) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to load manifest from {:?}: {}", pkg_dir, e);
                    return ExitCode::from(1);
                }
            };

            // 2. Load private signing key
            let signing_key = match load_author_signing_key(&key_path) {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Failed to load private key from {:?}: {}", key_path, e);
                    return ExitCode::from(1);
                }
            };

            // 3. Compute tree hash
            let tree_hash = match compute_package_tree_hash(&pkg_dir, &manifest) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Failed to compute canonical tree hash: {}", e);
                    return ExitCode::from(1);
                }
            };

            // 4. Construct identity
            let identity = SignerIdentity {
                author_id: author_id.unwrap_or_else(|| manifest.id.clone()),
                name: author_name.unwrap_or_else(|| manifest.author.clone()),
                handle: author_handle,
            };

            // 5. Sign tree hash with domain separation
            let sig_meta =
                match sign_package_tree_hash(&signing_key, &manifest.id, &tree_hash, identity) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Signing error: {}", e);
                        return ExitCode::from(1);
                    }
                };

            // 6. Write release.sig
            let sig_path = pkg_dir.join("release.sig");
            let json_data = match serde_json::to_string_pretty(&sig_meta) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("Failed to serialize signature metadata: {}", e);
                    return ExitCode::from(1);
                }
            };

            if let Err(e) = fs::write(&sig_path, json_data) {
                eprintln!("Failed to write release.sig to {:?}: {}", sig_path, e);
                return ExitCode::from(1);
            }

            println!("==================================================");
            println!("Package Signed Successfully");
            println!("Package:    {} v{}", manifest.id, manifest.version);
            println!("Tree Hash:  {}", tree_hash);
            println!("Key ID:     {}", sig_meta.key_id);
            println!("Signer:     {}", sig_meta.signer_identity.name);
            println!("Signature:  {:?}", sig_path);
            println!("==================================================");
            ExitCode::SUCCESS
        }
        "verify-package" => {
            if args.len() < 3 {
                eprintln!("Usage: ryzora-ci verify-package <pkg-dir>");
                return ExitCode::from(1);
            }
            let pkg_dir = PathBuf::from(&args[2]);
            let manifest = match load_package_manifest(&pkg_dir) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to load manifest: {}", e);
                    return ExitCode::from(1);
                }
            };

            let trust_store = TrustStore::load_default();
            match evaluate_package_directory_crypto(&pkg_dir, &manifest, None, &trust_store) {
                Ok(eval) => {
                    println!("==================================================");
                    println!("Package Cryptographic Verification: {:?}", eval.status);
                    println!("Valid:      {}", eval.is_valid);
                    println!("Trusted:    {}", eval.is_trusted);
                    println!("Can Install:{}", eval.can_install);
                    if let Some(kid) = eval.key_id {
                        println!("Key ID:     {}", kid);
                    }
                    if let Some(name) = eval.signer_name {
                        println!("Signer:     {}", name);
                    }
                    if let Some(err) = eval.error_message {
                        println!("Note/Error: {}", err);
                    }
                    println!("==================================================");
                    if eval.is_valid {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(e) => {
                    eprintln!("Verification failed: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("Unknown command: {}", other);
            print_usage();
            ExitCode::from(1)
        }
    }
}
