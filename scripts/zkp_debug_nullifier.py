import hashlib

# BN254 modulus (Fr)
BN254_MODULUS = int("21888242871839275222246405745257275088548364400416034343698204186575808495617")

def hex_to_int(hexstr):
    return int(hexstr, 16)

def rust_fr_from_scope(scope: str) -> int:
    sha = hashlib.sha256()
    sha.update(scope.encode('utf-8'))
    digest = sha.digest()
    be_int = int.from_bytes(digest, byteorder='big')
    return be_int % BN254_MODULUS

def poseidon_hash(inputs):
    # Placeholder: you need a Poseidon implementation compatible with Arkworks
    # For now, just print the inputs
    print(f"Poseidon hash inputs: {[hex(i) for i in inputs]}")
    print("Poseidon hash: [implement with pyPoseidon or similar]")
    return None

if __name__ == "__main__":
    scope = "support"
    secret_hex = "0000000000000000000000000000000000000000000000000000000000000000"
    secret_int = hex_to_int(secret_hex)
    scope_fr = rust_fr_from_scope(scope)
    print(f"scope_fr: {hex(scope_fr)}")
    print(f"secret_int: {hex(secret_int)}")
    # Poseidon hash (scope_fr, secret_int)
    poseidon_hash([scope_fr, secret_int])
