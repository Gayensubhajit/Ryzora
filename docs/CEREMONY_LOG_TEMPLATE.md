# Ryzora Official Root Key Ceremony Log (Template)

**Date of Ceremony:** [YYYY-MM-DD]  
**Location:** [Physical Air-Gapped Facility / Isolated Environment]  
**Security Level:** Air-Gapped Level 3 (No wireless, no network, read-only live OS)  

---

## 1. Witness Attendance & Verification

| Role | Name | Organization / Handle | Signature / Verification |
|---|---|---|---|
| Ceremony Administrator | [Maintainer 1] | Ryzora Core Team | [Signed] |
| Security Witness | [Maintainer 2] | Ryzora Security | [Signed] |
| Independent Observer | [Observer] | External / Community | [Signed] |

---

## 2. Air-Gapped Environment Attestation

- [ ] Booted from clean, audited Live Linux medium (read-only)
- [ ] Physical verification of zero active network devices (`ip link` shows only `lo`)
- [ ] Bluetooth and RF hardware disabled / absent
- [ ] Audited `ryzora-ci` binary hash matched reference build:  
  `SHA-256 (ryzora-ci): [hash]`

---

## 3. Key Generation Artifacts

**Command Executed:**
```bash
./ryzora-ci keygen ryzora-prod-root-[YEAR]
```

### Public Key Information:
- **Identifier:** `ryzora-prod-root-[YEAR]`
- **Algorithm:** Ed25519 (RFC 8032)
- **Public Key (Hex):** `[64-hex-characters]`
- **Deterministic Key ID / Fingerprint:** `ed25519:[16-hex-characters]`

---

## 4. Custody & Storage Verification

- [ ] Private key (`ryzora-prod-root-[YEAR].key`) copied to Hardware-Encrypted USB Drive #1 (Primary)
- [ ] Private key (`ryzora-prod-root-[YEAR].key`) copied to Hardware-Encrypted USB Drive #2 (Backup)
- [ ] File permissions verified: `0600`
- [ ] Zero instances of private key written to permanent storage or git repository
- [ ] Volatile RAM wiped via shutdown (`sync; poweroff`)
- [ ] Primary USB secured in Safe Deposit Box A
- [ ] Backup USB secured in Safe Deposit Box B

---

## 5. Client Provisioning Value

To compile official distribution binaries recognizing this root key:
```bash
export RYZORA_OFFICIAL_ROOT_KEY="[64-hex-characters]"
```
