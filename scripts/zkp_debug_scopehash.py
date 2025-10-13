import hashlib

# BN254 modulus (Fr)
BN254_MODULUS = int("21888242871839275222246405745257275088548364400416034343698204186575808495617")

def rust_fr_from_scope(scope: str) -> str:
    # Step 1: SHA256 hash of the scope string (UTF-8)
    sha = hashlib.sha256()
    sha.update(scope.encode('utf-8'))
    digest = sha.digest()  # 32 bytes
    print(f"SHA256(scope) hex: {digest.hex()}")

    # Step 2: Interpret as big-endian integer
    be_int = int.from_bytes(digest, byteorder='big')
    print(f"SHA256(scope) as int: {be_int}")

    # Step 3: Reduce mod BN254
    fr_int = be_int % BN254_MODULUS
    print(f"Fr (mod BN254): {fr_int}")

    # Step 4: Encode as 32-byte big-endian hex (like Rust fr_to_hex)
    fr_bytes = fr_int.to_bytes(32, byteorder='big')
    fr_hex = fr_bytes.hex()
    print(f"Fr hex (for public_inputs): {fr_hex}")
    return fr_hex

if __name__ == "__main__":
    scope = "support"
    print(f"Testing scope: '{scope}'")
    rust_fr_from_scope(scope)
