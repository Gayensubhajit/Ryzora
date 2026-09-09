use ryzora_lib::crypto::TrustStore;
use ryzora_lib::ingestion::{ingest_submission_into_repository, run_ci_audit};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn print_usage() {
    eprintln!(
        "Usage: ryzora-ci audit <submission-dir> [--repo <repo-dir>] [--markdown-out <file>]"
    );
    eprintln!("       ryzora-ci ingest <submission-dir> <repo-dir>");
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return ExitCode::from(1);
    }

    let command = &args[1];
    match command.as_str() {
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
                        eprintln!(
                            "
Errors:"
                        );
                        for err in &report.errors {
                            eprintln!("  - {}", err);
                        }
                    }
                    if !report.warnings.is_empty() {
                        println!(
                            "
Warnings:"
                        );
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
                        println!(
                            "
Markdown comment generated: {:?}",
                            md_path
                        );
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
        other => {
            eprintln!("Unknown command: {}", other);
            print_usage();
            ExitCode::from(1)
        }
    }
}
