import re
from pathlib import Path

# Load errors.log
log_path = Path("errors.log")
with log_path.open("r", encoding="utf-8") as f:
    lines = [line.rstrip("\n") for line in f]


# Parsed values
proof_bytes = None
server_proof = None

public_inputs = []
fr_values = []
server_vk_hash = None
cli_vk_hash = None
poseidon_cli = None
poseidon_server = None

reading_inputs = False
reading_fr = False



for line in lines:
    # Proof bytes (wallet-cli)
    if line.startswith("Proof bytes:"):
        proof_bytes = line.split(":", 1)[1].strip()

    # Proof bytes (server)
    elif line.startswith("[verify_zkp_handler] Received proof_bytes"):
        server_proof = line.split(":", 2)[-1].strip()

    # Public inputs (server hex form)
    elif line.startswith("[verify_zkp_handler] Received public_inputs"):
        reading_inputs = True
        reading_fr = False
        continue

    elif reading_inputs:
        # Only collect lines that are valid 64-char hex
        hex_candidate = line.strip().split(" ", 1)[-1]
        if re.fullmatch(r"[0-9a-fA-F]{64}", hex_candidate):
            public_inputs.append(hex_candidate)
        # End reading if not a valid input
        elif line.strip().startswith("[") or line.strip().startswith("Converted public_inputs to Fr:"):
            reading_inputs = False
            reading_fr = line.strip().startswith("Converted public_inputs to Fr:")
            continue

    # Fr values: match both formats
    if re.match(r"Fr values: \[\d\] BigInt\(\[", line.strip()):
        fr_values.append(line.strip()[len("Fr values: "):])

    if line.startswith("[verify_zkp_handler] Converted public_inputs to Fr:"):
        reading_fr = True
        continue

    elif reading_fr:
        # Only collect lines that look like BigInt
        if re.match(r"\[\d\] BigInt\(\[", line.strip()):
            fr_values.append(line.strip())
        # End reading if not a valid Fr value
        elif not line.strip():
            reading_fr = False

    # VK hash (server)
    if line.startswith("[verify_zkp_handler] VK hash:"):
        server_vk_hash = line.split(":", 1)[-1].strip()
    # VK hash (cli)
    if line.startswith("VK hash (serveur):"):
        cli_vk_hash = line.split(":", 1)[-1].strip()
    # Poseidon params hash (cli)
    if line.startswith("Poseidon params hash (cli):"):
        poseidon_cli = line.split(":", 1)[-1].strip()
    # Poseidon params hash (server)
    if line.startswith("Poseidon params hash (server):") or line.startswith("Poseidon params hash (serveur):"):
        poseidon_server = line.split(":", 1)[-1].strip()

# ---- Cross-checks ----

if server_proof and proof_bytes:
    print("Cross-check: Proof bytes match:", proof_bytes == server_proof)
    assert proof_bytes == server_proof, "Proof bytes do not match between wallet-cli and server"


print("Proof bytes:", proof_bytes)
print("Public inputs:", public_inputs)
print("Fr values:", fr_values)
print("VK hash (server):", server_vk_hash)
print("VK hash (cli):", cli_vk_hash)
print("Poseidon params hash (server):", poseidon_server)
print("Poseidon params hash (cli):", poseidon_cli)

if server_vk_hash and cli_vk_hash:
    print("VK hash match:", server_vk_hash == cli_vk_hash)
    assert server_vk_hash == cli_vk_hash, "VK hash mismatch between server and wallet-cli"
if poseidon_server and poseidon_cli:
    print("Poseidon params hash match:", poseidon_server == poseidon_cli)
    assert poseidon_server == poseidon_cli, "Poseidon params hash mismatch between server and wallet-cli"

# Test 1: Public inputs validity
assert len(public_inputs) == 3, "Expected 3 public inputs"
for i, hex_str in enumerate(public_inputs):
    assert len(hex_str) == 64, f"Input {i} not 32 bytes"
    assert re.fullmatch(r"[0-9a-fA-F]{64}", hex_str), f"Input {i} not valid hex"
print("All public inputs are 32 bytes, valid hex, and in correct order.")

# Test 2: Proof bytes validity
assert proof_bytes is not None and len(proof_bytes) > 0, "Proof bytes missing or empty"
assert re.fullmatch(r"[0-9a-fA-F]+", proof_bytes), "Proof bytes not valid hex"
print("Proof bytes are present and valid hex.")

# Test 3: Fr value format
assert len(fr_values) == 3, "Expected 3 Fr values"
for i, fr_str in enumerate(fr_values):
    assert fr_str.startswith(f"[{i}] BigInt(["), f"Fr value {i} format incorrect"
    nums = re.findall(r"BigInt\(\[(.*?)\]\)", fr_str)
    assert nums, f"Fr value {i} missing BigInt array"
    arr = [int(x.strip()) for x in nums[0].split(",")]
    assert len(arr) == 4, f"Fr value {i} BigInt array not length 4"
print("All Fr values are valid BigInt arrays of length 4.")
