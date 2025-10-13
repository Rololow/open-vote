use std::fs;
use std::path::{Path, PathBuf};

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(read) = fs::read_dir(dir) {
        for entry in read.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_rs_files(&p, out);
            } else if p.extension().map(|e| e == "rs").unwrap_or(false) {
                out.push(p);
            }
        }
    }
}

#[test]
fn no_sensitive_identity_fields_in_log_lines() {
    // Scan only server source files for logging macros leaking VC internals
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);

    // Macros that write to logs/stdout in server code
    let log_markers = [
        "info!(", "warn!(", "error!(", "debug!(", "trace!(",
        "tracing::info!", "tracing::warn!", "tracing::error!", "tracing::debug!", "tracing::trace!",
        "println!(", "eprintln!("
    ];
    // VC fields we must NOT log from server: logging these would imply leaking raw VC
    let sensitive_tokens = [
        "credentialSubject",
        "\"proof\"", // exact JSON key
        "proofValue",
        "verificationMethod",
    ];

    let mut violations: Vec<(PathBuf, usize, String)> = Vec::new();
    for file in files {
        if let Ok(content) = fs::read_to_string(&file) {
            for (idx, line) in content.lines().enumerate() {
                // Quick filter: consider only lines that contain a logging macro token
                if !log_markers.iter().any(|m| line.contains(m)) { continue; }
                // If line contains any sensitive token, flag it
                if let Some(tok) = sensitive_tokens.iter().find(|tok| line.contains(*tok)) {
                    violations.push((file.clone(), idx + 1, format!("{} => {}", tok, line.trim())));
                }
            }
        }
    }

    if !violations.is_empty() {
        let mut msg = String::from("Sensitive VC fields found in server log lines (should not be logged):\n");
        for (path, line, ctx) in violations {
            msg.push_str(&format!("- {}:{} :: {}\n", path.display(), line, ctx));
        }
        panic!("{}", msg);
    }
}

#[test]
fn no_private_key_in_migrations_schema() {
    // Ensure DB migrations never create/store private keys
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("crate parent");
    let migrations_dir = workspace_root.join("migrations");
    if !migrations_dir.exists() { return; }

    let mut offenders = Vec::new();
    for entry in fs::read_dir(&migrations_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().map(|e| e == "sql").unwrap_or(false) {
            let content = fs::read_to_string(entry.path()).unwrap_or_default();
            // lowercase content for robust search
            let lc = content.to_lowercase();
            if lc.contains("private_key") || lc.contains("secret_key") || lc.contains("seed") {
                offenders.push(entry.path());
            }
        }
    }
    if !offenders.is_empty() {
        let list = offenders.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n - ");
        panic!("DB migrations reference private/secret key material; must not persist!\n - {}", list);
    }
}
