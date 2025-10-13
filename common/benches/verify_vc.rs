use criterion::{criterion_group, criterion_main, Criterion, black_box, BatchSize};
use sha2::{Digest, Sha256};
use bs58;
use common::identity::{CitizenCredentialWrapper, canonical_json_str};
use crypto_lib::{KeyPair, PublicKey, Signature};

// Minimal representative VC (without proof). We'll attach a proof-like structure ourselves for realism,
// but the measured path focuses on canonicalization + Ed25519 verify over canonical unsigned JSON.
const SAMPLE_VC_UNSIGNED: &str = r#"{"issuer":"did:key:zISSUER","credentialSubject":{"id":"did:key:zSUBJ","country":"FR","over18":true},"type":["VerifiableCredential","CitizenCredential"],"@context":["https://www.w3.org/2018/credentials/v1"],"metadata":{"roles":["citizen"],"version":1}}"#;

fn build_did_key_from_pubkey(pk_bytes: &[u8;32]) -> String {
    // did:key for ed25519: multicodec header 0xED 0x01 + 32-byte key, base58btc with 'z'
    let mut data = Vec::with_capacity(34);
    data.push(0xED);
    data.push(0x01);
    data.extend_from_slice(pk_bytes);
    format!("did:key:z{}", bs58::encode(data).into_string())
}

fn build_verification_method(issuer_did: &str) -> String { format!("{}#controller", issuer_did) }

fn bench_verify_vc(c: &mut Criterion) {
    // Prepare a deterministic keypair (fixed seed) for repeatable benches
    let seed: [u8;32] = [7u8; 32];
    let kp = KeyPair::from_seed(&seed);
    let pub_bytes = kp.public_key().to_bytes();
    let issuer_did = build_did_key_from_pubkey(&pub_bytes);
    let verification_method = build_verification_method(&issuer_did);

    // Construct an unsigned VC JSON from the constant and ensure canonical form is deterministic
    let wrapper = CitizenCredentialWrapper::parse(SAMPLE_VC_UNSIGNED).expect("parse vc");
    let raw_unsigned = wrapper.raw_credential_json.clone();

    // Precompute canonical and digest
    let canonical = canonical_json_str(&raw_unsigned).expect("canon");
    let digest = Sha256::digest(canonical.as_bytes());
    let digest_hex = hex::encode(digest);

    // Sign canonical bytes (mimics server-side signing) and encode as z-base58btc proofValue
    let signature = kp.sign(canonical.as_bytes());
    let sig_bytes = signature.to_bytes();
    let proof_value = format!("z{}", bs58::encode(&sig_bytes).into_string());

    // Public key object for verification path
    let pubkey = PublicKey::from_bytes(&pub_bytes).expect("pub from bytes");

    // 1) Canonicalization cost (baseline)
    c.bench_function("vc_verify/canonicalize_unsigned", |b| {
        b.iter_batched(
            || raw_unsigned.clone(),
            |raw| {
                let cjson = canonical_json_str(black_box(&raw)).expect("canon");
                black_box(cjson);
            },
            BatchSize::SmallInput,
        )
    });

    // 2) Digest recomputation cost
    c.bench_function("vc_verify/digest_sha256_hex", |b| {
        b.iter(|| {
            let d = Sha256::digest(black_box(canonical.as_bytes()));
            let hx = hex::encode(d);
            // compare to precomputed to mimic endpoint behavior when proof.digest is present
            assert_eq!(hx, digest_hex);
            black_box(hx);
        })
    });

    // 3) Ed25519 verify cost over canonical bytes
    c.bench_function("vc_verify/ed25519_verify", |b| {
        b.iter(|| {
            let sig = Signature::from_bytes(black_box(&sig_bytes)).expect("sig bytes");
            pubkey.verify(black_box(canonical.as_bytes()), &sig).expect("verify");
        })
    });

    // 4) End-to-end verify bundle (canonicalize + digest + verify) to observe combined cost
    c.bench_function("vc_verify/e2e_canon_digest_verify", |b| {
        b.iter_batched(
            || raw_unsigned.clone(),
            |raw| {
                let cjson = canonical_json_str(&raw).expect("canon");
                let d = Sha256::digest(cjson.as_bytes());
                let hx = hex::encode(d);
                assert_eq!(hx, digest_hex);
                let sig = Signature::from_bytes(&sig_bytes).expect("sig bytes");
                pubkey.verify(cjson.as_bytes(), &sig).expect("verify");
            },
            BatchSize::SmallInput,
        )
    });

    // 5) Negative verify: flip one bit and expect failure (verifier work still performed)
    let mut tampered = sig_bytes;
    tampered[0] ^= 0x01;
    c.bench_function("vc_verify/ed25519_verify_invalid", |b| {
        b.iter(|| {
            let sig = Signature::from_bytes(black_box(&tampered)).expect("sig bytes");
            let res = pubkey.verify(black_box(canonical.as_bytes()), &sig);
            assert!(res.is_err());
        })
    });

    // Bench also with verificationMethod string building (tiny overhead, but included for realism)
    c.bench_function("vc_verify/build_verification_method", |b| {
        b.iter(|| {
            let vm = build_verification_method(black_box(&issuer_did));
            assert_eq!(vm, verification_method);
            black_box(vm);
        })
    });

    // Track proof serialization size to ensure stable inputs across runs
    let _pv_len = proof_value.len();
    let _issuer_len = issuer_did.len();
    let _vm_len = verification_method.len();
    criterion::black_box((_pv_len, _issuer_len, _vm_len));
}

criterion_group!(benches, bench_verify_vc);
criterion_main!(benches);
