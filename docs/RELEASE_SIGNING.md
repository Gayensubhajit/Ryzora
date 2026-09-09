# Ryzora Release Signing & Cryptographic Specification

## 1. Cryptographic Standards

Ryzora enforces pure, declarative verification for all packages and official distributions:
- **Algorithm**: `Ed25519` (RFC 8032) exclusively. Non-Ed25519 algorithms are rejected.
- **Tree Hash**: SHA-256 canonical directory tree hash computed over sorted relative file paths and byte contents.
- **Domain Separation**: All release signatures prepend the protocol domain identifier:
  `ryzora-v1:<package_id>:<canonical_tree_hash>`
- **Detached Signature**: Stored in JSON format as `release.sig` at the package root.

---

## 2. Signature Schema (`release.sig`)

```json
{
  "schema_version": 1,
  "algorithm": "ed25519",
  "key_id": "ed25519:61aede6f9b62e7af",
  "public_key": "d1a9eda16bd08fae78c922688650008b4bce235a1171044f3b68d1cb109dc0fb",
  "signed_tree_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "signature": "3b6a27bc4f...",
  "signer_identity": {
    "author_id": "hypr-catppuccin-mocha",
    "name": "Catppuccin Maintainers",
    "handle": "@catppuccin"
  },
  "signed_at": "1773000000"
}
```

---

## 3. Package Signing Workflow

Package authors and release managers sign packages using `ryzora-ci`:

### Step 1: Generate an Author Keypair
```bash
ryzora-ci keygen ~/.config/ryzora/keys/my-author-key
```
This produces:
- `~/.config/ryzora/keys/my-author-key.pub`
- `~/.config/ryzora/keys/my-author-key.key` (enforced mode `0600`)
- `~/.config/ryzora/keys/my-author-key.id`

### Step 2: Sign the Package Directory
```bash
ryzora-ci sign-package ./packages/my-rice ~/.config/ryzora/keys/my-author-key.key \
  --author-name "Alice Creator" \
  --author-handle "@alice"
```
This computes the canonical tree hash, signs the domain-separated payload, and writes `./packages/my-rice/release.sig`.

### Step 3: Verify the Signature
```bash
ryzora-ci verify-package ./packages/my-rice
```

---

## 4. Trust Tiers

| Tier | Evaluation Criteria | UI Badge |
|---|---|---|
| **Official** | Signed with recognized Official Root Key. | 🛡️ Official Core |
| **AuthorVerified** | Signed by author key registered in trusted keyring. | 👤 Verified Creator |
| **SelfSignedUnvetted** | Valid signature, but author key is unknown to trust store. | ⚠️ Community Unvetted |
| **Unsigned** | No `release.sig` present; files valid according to manifest. | 📦 Unsigned Community |
| **RevokedKey** | Signed with a key listed in revoked keys database. | 🚫 Revoked (Blocked) |
| **SignatureFailed** | Mathematical verification failed or tree hash mismatch. | ❌ Tampered (Blocked) |
