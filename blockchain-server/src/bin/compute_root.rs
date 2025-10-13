use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}

fn merkle_root(mut leaves: Vec<[u8; 32]>) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    while leaves.len() > 1 {
        let mut next = Vec::with_capacity((leaves.len() + 1) / 2);
        let mut i = 0;
        while i < leaves.len() {
            let left = leaves[i];
            let right = if i + 1 < leaves.len() { leaves[i + 1] } else { left };
            let mut buf = Vec::with_capacity(64);
            buf.extend_from_slice(&left);
            buf.extend_from_slice(&right);
            next.push(sha256(&buf));
            i += 2;
        }
        leaves = next;
    }
    leaves[0]
}

fn read_hex_hashes(path: &str) -> anyhow::Result<Vec<[u8; 32]>> {
    let f = File::open(path)?;
    let rdr = BufReader::new(f);
    let mut out = Vec::new();
    for line in rdr.lines() {
        let line = line?;
        let s = line.trim();
        if s.is_empty() { continue; }
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 { anyhow::bail!("invalid hash length in log: {} (expected 32 bytes)", bytes.len()); }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        out.push(arr);
    }
    Ok(out)
}

fn write_root(path: &str, root: &[u8; 32]) -> anyhow::Result<()> {
    std::fs::write(path, hex::encode(root))?;
    Ok(())
}

fn print_usage(program: &str) {
    eprintln!(
        "Usage:\n  {program} [<commitments.log>] [--out <path>]\n\nOptions:\n  -h, --help           Show this help and exit\n  --out <path>         Write the hex Merkle root to the given file as well\n\nNotes:\n  • If no input file is provided, defaults to '<crate>/data/commitments.log'.\n  • Each non-empty line in the input must be a 32-byte hash encoded as hex.",
        program = program
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_root_even_leaves() {
        // Two simple 32-byte leaves: 'a' * 32 and 'b' * 32
        let a = [b'a'; 32];
        let b = [b'b'; 32];
        let expected = {
            let mut buf = Vec::with_capacity(64);
            buf.extend_from_slice(&a);
            buf.extend_from_slice(&b);
            sha256(&buf)
        };
        let got = merkle_root(vec![a, b]);
        assert_eq!(got, expected, "even-leaf merkle root should hash left||right once");
    }

    #[test]
    fn merkle_root_odd_leaves() {
        // Three leaves: 'a', 'b', 'c' (32 bytes each)
        let a = [b'a'; 32];
        let b = [b'b'; 32];
        let c = [b'c'; 32];
        // Level 1
        let l0 = {
            let mut buf = Vec::with_capacity(64);
            buf.extend_from_slice(&a);
            buf.extend_from_slice(&b);
            sha256(&buf)
        };
        let l1 = {
            // Odd leaf duplicates the last element (c||c)
            let mut buf = Vec::with_capacity(64);
            buf.extend_from_slice(&c);
            buf.extend_from_slice(&c);
            sha256(&buf)
        };
        // Root
        let expected = {
            let mut buf = Vec::with_capacity(64);
            buf.extend_from_slice(&l0);
            buf.extend_from_slice(&l1);
            sha256(&buf)
        };
        let got = merkle_root(vec![a, b, c]);
        assert_eq!(got, expected, "odd-leaf merkle root should duplicate the last leaf at the level");
    }
}

fn main() -> anyhow::Result<()> {
    // Simple CLI parsing without extra deps
    let mut args = std::env::args().skip(1);
    let mut input_path: Option<String> = None;
    let mut out_path: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                let program = std::env::args().next().unwrap_or_else(|| "compute_root".to_string());
                print_usage(&program);
                return Ok(());
            }
            "--out" => {
                if let Some(p) = args.next() {
                    out_path = Some(p);
                } else {
                    anyhow::bail!("--out requires a path argument");
                }
            }
            s if s.starts_with('-') => {
                anyhow::bail!("unknown flag: {s}");
            }
            s => {
                if input_path.is_none() {
                    input_path = Some(s.to_string());
                } else {
                    anyhow::bail!("unexpected extra positional argument: {s}");
                }
            }
        }
    }

    // Default to crate-relative data/commitments.log so it works regardless of CWD
    let log_path = input_path.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data").join("commitments.log").to_string_lossy().to_string()
    });

    let leaves = read_hex_hashes(&log_path)?;
    let root = merkle_root(leaves);
    let hex_root = hex::encode(root);
    println!("merkle_root={}", hex_root);

    if let Some(p) = out_path {
        write_root(&p, &root)?;
        eprintln!("wrote root to {}", p);
    }
    Ok(())
}
