use criterion::{criterion_group, criterion_main, Criterion, black_box, BatchSize};
use common::identity::{CitizenCredentialWrapper, compute_credential_hash, canonical_json_str};

// VC JSON stabilisé pour benchmark (structure représentative de taille modérée)
const SAMPLE_VC: &str = r#"{"issuer":"did:key:zISSUER","credentialSubject":{"id":"did:key:zSUBJ","country":"FR","over18":true},"type":["VerifiableCredential","CitizenCredential"],"@context":["https://www.w3.org/2018/credentials/v1"],"metadata":{"roles":["citizen"],"version":1}}"#;

fn bench_hash_and_canonical(c: &mut Criterion) {
    // Pré-parse unique hors boucle
    let cred = CitizenCredentialWrapper::parse(SAMPLE_VC).expect("parse sample vc");
    let raw = cred.raw_credential_json.clone();

    c.bench_function("identity_canonical_json", |b| {
        b.iter_batched(
            || raw.clone(),
            |r| {
                let canon = canonical_json_str(black_box(&r)).expect("canon");
                black_box(canon);
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("identity_hash_compute", |b| {
        b.iter(|| {
            let h = compute_credential_hash(black_box(&cred)).expect("hash");
            black_box(h);
        })
    });
}

criterion_group!(benches, bench_hash_and_canonical);
criterion_main!(benches);
