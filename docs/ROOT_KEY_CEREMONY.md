# Ryzora Official Root Key Ceremony & Key Management Protocol

## 1. Overview & Threat Model

Ryzora employs a multi-tiered cryptographic trust model rooted in pure Ed25519 public-key cryptography. In this architecture:
- **Official Core Root Key**: Authoritative trust anchor that designates packages vetted and published by the official Ryzora project.
- **Author Keys**: Creator-generated Ed25519 keys used to sign packages (`release.sig`). Authors can be vetted into the `AuthorVerified` tier via the Trust Store.
- **Package Integrity**: Every release contains a detached signature over a domain-separated canonical tree hash (`ryzora-v1:<package_id>:<sha256_tree_hash>`).

### Security Invariant
**The production Official Root private key is NEVER committed to Git, stored on internet-connected build servers, or embedded in repository source code.**

The Ryzora client source code embeds only the **public key** and its fingerprint. In local development and test builds, Ryzora utilizes a dedicated development trust key (`RYZORA_DEV_TEST_ROOT_PUBKEY_HEX`). For official distribution releases, the authentic production root key is supplied at build time via `RYZORA_OFFICIAL_ROOT_KEY`.

---



---

---

## 1.1 Architectural Separation: Root Key vs. Release Signing Key

> [!IMPORTANT]
> **Cryptographic Separation Invariant:**
> **The production Official Root Key is NOT intended to sign release artifacts directly.**
>
> - **Production Official Root Key (`RYZORA_OFFICIAL_ROOT_KEY`)**:
>   The ultimate trust anchor compiled into the Ryzora client (`TrustStore`). Used exclusively to evaluate and verify that package releases (`release.sig`) carry the authentic `Official` tier. The private key remains indefinitely air-gapped and offline in encrypted hardware storage.
>
> - **Release Signing Key (`RYZORA_RELEASE_SIGNING_KEY` / `RYZORA_RELEASE_PUBKEY`)**:
>   An operational Ed25519 signing keypair used solely to generate detached cryptographic signatures (`SHA256SUMS.txt.sig`) over compiled distribution binaries, AppImages, Debian packages, and release archives. This key can be rotated across release series or CI environments without compromising the immutable air-gapped root trust anchor.
>
> Under NO circumstances should automated AI coding agents generate, store, or handle the real production root private key. The actual root key ceremony is an operator-controlled air-gapped procedure.

---

## 2. Key Ceremony Preparation

The Official Root Key must be generated in a clean, audited ceremony on an air-gapped machine.

### Equipment Requirements:
1. **Air-Gapped Machine**: Hardware with all wireless/Bluetooth chips physically disabled or removed, booted into a clean read-only Linux Live OS (e.g. Tails or Debian Live).
2. **Encrypted Media**: Two identical, high-durability hardware-encrypted USB flash drives (Primary and Backup).
3. **Physical Custody**: At least two authorized project maintainers present to verify the ceremony.

---

## 3. Ceremony Execution Procedure

### Step 1: Boot Environment
Boot the air-gapped machine from the live medium. Confirm no network interfaces exist:
```bash
ip link
# Verify only 'lo' is listed
```

### Step 2: Build or Transfer `ryzora-ci`
Mount a read-only transfer medium containing the pre-compiled `ryzora-ci` binary (compiled from clean git tag) and verify its SHA-256 hash.

### Step 3: Generate the Root Keypair
Execute the audited key generation command:
```bash
./ryzora-ci keygen ryzora-prod-root-2026
```

This generates three files with strict Unix permissions:
- `ryzora-prod-root-2026.pub` (64-character hex public key, mode `0644`)
- `ryzora-prod-root-2026.key` (64-character hex private key, mode `0600`)
- `ryzora-prod-root-2026.id`  (Canonical fingerprint `ed25519:<hash>`, mode `0644`)

### Step 4: Verify Public Key Fingerprint
Verify the fingerprint deterministically:
```bash
./ryzora-ci fingerprint ryzora-prod-root-2026.pub
```
Record the fingerprint in the ceremony log signed by all witnesses.

### Step 5: Secure Offline Storage
1. Copy `ryzora-prod-root-2026.key` to both Primary and Backup encrypted USB drives.
2. Store the USB drives in separate physical safe deposit boxes or high-security safes.
3. Securely wipe RAM and volatile storage before rebooting the air-gapped host:
   ```bash
   sync; poweroff
   ```

---

## 4. Provisioning the Production Root Key

### In GitHub Release Builds
The public key hex string is configured as a repository secret or compile-time variable:
```bash
export RYZORA_OFFICIAL_ROOT_KEY="<public_key_hex>"
cargo build --manifest-path src-tauri/Cargo.toml --release
```

### Unconfigured Production Safety Guarantee
If `RYZORA_OFFICIAL_ROOT_KEY` is not provided during a production build:
- The `official_core_keys` trust list is strictly **empty**.
- Under no circumstances can any package evaluate to `Official`.
- Unsigned or self-signed community packages can still be installed as `Community (Unvetted)` or `SelfSignedUnvetted` with user consent.
- `TrustStore::is_production_ready()` returns `false`.

---

## 5. Key Rotation & Revocation Protocol

### Root Key Rotation:
When a root key is scheduled for rotation:
1. Conduct a new root key ceremony for the successor keypair (e.g. `ryzora-prod-root-2027`).
2. Add the successor public key to `RYZORA_RECOGNIZED_OFFICIAL_ROOT_KEYS` in `src-tauri/src/crypto.rs` alongside the existing root key.
3. Release a client update. Both old and new official packages validate during the transition window.
4. After the transition window, retire the predecessor key.

### Emergency Revocation:
If private key compromise is suspected:
1. Issue an immediate client patch adding the compromised key fingerprint to `get_revoked_keys_file()`.
2. Packages signed by the compromised key are immediately quarantined and refused installation with `CryptographicStatus::RevokedKey`.
