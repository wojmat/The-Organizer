# Security Documentation

This document describes the security model, cryptographic choices, threat model, and known limitations of The Organizer password manager.

---

## Table of Contents

1. [Threat Model](#threat-model)
2. [Cryptographic Primitives](#cryptographic-primitives)
3. [Security Features](#security-features)
4. [Known Limitations](#known-limitations)
5. [Best Practices](#best-practices)
6. [Responsible Disclosure](#responsible-disclosure)

---

## Threat Model

### What We Protect Against ✅

**1. Vault File Theft**
- **Scenario**: Attacker gains access to vault.dat file through backup theft, cloud sync compromise, or disk recovery
- **Protection**: Strong encryption (XChaCha20-Poly1305) renders the vault useless without the master password
- **Strength**: Argon2id makes brute force attacks computationally expensive (64 MiB memory per attempt)

**2. Memory Dumps**
- **Scenario**: Attacker captures memory snapshot while vault is locked
- **Protection**: Sensitive data (keys, passwords, plaintext) is explicitly zeroized when the vault locks
- **Implementation**: Zeroize crate overwrites memory with zeros before deallocation

**3. Brute Force Password Attacks**
- **Scenario**: Attacker attempts to guess master password through repeated unlock attempts
- **Protection**:
  - Argon2id makes each guess computationally expensive (~200ms on modern hardware)
  - Rate limiting: After 5 failed attempts, enforces 30-second cooldown
  - No online attack vector (local-only application)

**4. Replay Attacks**
- **Scenario**: Attacker captures and replays encrypted vault data
- **Protection**: Fresh 24-byte nonce generated for every save operation prevents ciphertext reuse
- **Authentication**: Poly1305 MAC prevents tampering with ciphertext

**5. Data Tampering**
- **Scenario**: Attacker modifies encrypted vault file
- **Protection**: Poly1305 authentication tag ensures integrity; modified vaults fail to decrypt

**6. Password Leakage via Clipboard**
- **Scenario**: Clipboard snooping malware reads passwords after copy
- **Protection**: Automatic clipboard clearing after 15 seconds
- **Limitation**: Cannot protect if app crashes before cleanup thread runs

### What We DON'T Protect Against ❌

**1. Keyloggers**
- **Scenario**: Malicious software records all keystrokes
- **Reality**: Master password entered via keyboard is vulnerable
- **Mitigation**: Not possible at application level; requires OS-level security (antivirus, trusted environment)

**2. Screen Recording / Screenshots**
- **Scenario**: Malware captures screen contents
- **Reality**: Passwords visible when entered in forms
- **Mitigation**: Not possible at application level

**3. Compromised Operating System**
- **Scenario**: Root-level malware with full system access
- **Reality**: Can read memory, intercept system calls, keylog, etc.
- **Mitigation**: Requires trusted OS; application-level security cannot defend against root compromise

**4. Physical Access to Unlocked App**
- **Scenario**: Attacker with physical access while vault is unlocked
- **Reality**: Can view/copy all passwords
- **Mitigation**: Auto-lock after 5 minutes of inactivity, manual lock button

**5. Weak Master Passwords**
- **Scenario**: User chooses easily guessable password (e.g., "password123")
- **Reality**: Argon2id slows brute force but cannot prevent weak password choices
- **Mitigation**: Minimum 10-character requirement (enforced), password strength feedback (future feature)

**6. Master Password Recovery**
- **Scenario**: User forgets master password
- **Reality**: Zero-knowledge design means password cannot be recovered
- **Mitigation**: None possible; emphasize password memorability during setup

---

## Cryptographic Primitives

### Key Derivation Function (KDF)

**Algorithm**: Argon2id (RFC 9106)

**Parameters**:
```rust
Memory cost:    64 MiB (65,536 KiB)
Time cost:      3 iterations
Parallelism:    1 thread
Output length:  32 bytes (256 bits)
```

**Rationale**:
- **Argon2id**: Winner of the Password Hashing Competition (2015)
- **Hybrid mode**: Combines data-dependent (Argon2i) and data-independent (Argon2d) memory access
- **Memory-hard**: 64 MiB requirement resists GPU/ASIC attacks
- **Time cost**: 3 iterations balance security and responsiveness (~200ms on typical hardware)
- **Parallelism**: Single-threaded for interactive use; prevents excessive CPU load

### Authenticated Encryption

**Algorithm**: XChaCha20-Poly1305 (IETF AEAD)

**Cipher**: XChaCha20
- Stream cipher based on ChaCha20
- **Extended nonce**: 24 bytes (vs 12 bytes for ChaCha20)
- **Advantage**: Larger nonce space eliminates birthday bound concerns
- **Speed**: Faster than AES on systems without AES-NI hardware

**Authentication**: Poly1305
- **MAC**: Message authentication code
- **Tag size**: 16 bytes (128-bit security)
- **Property**: Forgery probability < 2^-100 for reasonably sized messages

**Nonce Management**:
```
Nonce size:     24 bytes (192 bits)
Generation:     OsRng.fill_bytes() - cryptographically secure random
Uniqueness:     Fresh nonce per save operation
Collision risk: Negligible (2^-192 probability even after 2^96 operations)
```

### Random Number Generation

**Source**: `OsRng` from Rust's `rand` crate

**Entropy Source**:
- **Windows**: `BCryptGenRandom` (CNG API)
- **macOS/iOS**: `/dev/urandom` (seeded from entropy pool)
- **Linux**: `getrandom()` syscall or `/dev/urandom`

**Properties**:
- **Cryptographically secure**: Suitable for key/nonce generation
- **Non-blocking**: Always available (no entropy exhaustion)
- **OS-backed**: Leverages platform-specific CSPRNGs

---

## Security Features

### Memory Safety

**Zeroization**:
- All sensitive data (keys, passwords, plaintext) is explicitly overwritten with zeros before deallocation
- **Implementation**: Zeroize crate (`zeroize = "1"`)
- **Applied to**:
  - Master password (wrapped in `Zeroizing<String>`)
  - Derived keys (`Zeroizing<[u8; 32]>`)
  - Decrypted plaintext (JSON bytes)
  - Entry passwords (via `Entry::Drop` trait)

**Scope-based Cleanup**:
```rust
// Master password cleared when Zeroizing wrapper is dropped
let master = Zeroizing::new(master_password);
// ... use master ...
// Automatically zeroized at end of scope
```

### Session Management

**Auto-Lock**:
- **Trigger**: 5 minutes of inactivity (no mouse/keyboard events)
- **Polling**: Background thread checks every 10 seconds
- **Action**: Clears session (key + entries) from memory
- **Recovery**: None (requires re-entering master password)

**Manual Lock**:
- Explicit "Lock" button in UI
- Immediately clears session data
- Zeroizes sensitive fields

### Rate Limiting

**Failed Unlock Attempts**:
- **Threshold**: 5 failed attempts
- **Cooldown**: 30 seconds
- **Reset**: Counter cleared on successful unlock
- **State**: Tracked in `FailedAttemptTracker` (in-memory, lost on app restart)

### Vault Format Versioning

**Current Version**: `0x01`

**Format**:
```
[1 byte version][32 bytes salt][24 bytes nonce][ciphertext + 16-byte auth tag]
```

**Backward Compatibility**:
- Loader detects version byte vs. legacy format (no version)
- Legacy format supported (offsets adjusted for missing version byte)
- Saves always use latest version

---

## Known Limitations

### Clipboard Security

**Issue**: Clipboard clearing uses background thread
- **Risk**: If app crashes before thread runs (15 seconds), password persists indefinitely
- **Mitigation**: None (cross-platform clipboard APIs don't support deferred clearing)
- **Workaround**: Manually clear clipboard if app crashes

### Password Change

**Status**: Not implemented in v0.1.0
- **Impact**: Cannot rotate master password without creating new vault
- **Planned**: v0.2.0 will include `change_master_password` command

### Single-User, Single-Device

**Design**: Not intended for multi-user or sync scenarios
- **No conflict resolution**: Concurrent edits across devices will cause data loss
- **No merge**: Last write wins if vault.dat copied between devices

---

## Best Practices

### For Users

1. **Choose a Strong Master Password**:
   - Minimum 10 characters (enforced)
   - Use a passphrase: "correct horse battery staple" > "Tr0ub4dor&3"
   - Avoid personal information (birthdate, pet names)

2. **Backup Your Vault**:
   - Copy `vault.dat` to secure offline storage
   - Test restore procedure periodically
   - Vault file is useless without master password (safe to store in cloud)

3. **Use Auto-Lock**:
   - Let vault auto-lock when away from computer
   - Manually lock before closing laptop lid

4. **Protect Your Environment**:
   - Use encrypted disk (FileVault, BitLocker)
   - Keep OS and antivirus up to date
   - Avoid entering master password on untrusted devices

---

## Responsible Disclosure

If you discover a security vulnerability in The Organizer:

### DO:
1. **Email**: security@example.com (replace with actual contact)
2. **Include**:
   - Detailed description of the vulnerability
   - Steps to reproduce
   - Potential impact
3. **Wait**: Give us 90 days to fix before public disclosure

### DON'T:
- Publicly disclose before we've had time to fix it
- Exploit beyond proof-of-concept testing

---

## References

- **Argon2**: [RFC 9106](https://www.rfc-editor.org/rfc/rfc9106)
- **XChaCha20-Poly1305**: [draft-irtf-cfrg-xchacha](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-xchacha)
- **ChaCha20-Poly1305**: [RFC 8439](https://www.rfc-editor.org/rfc/rfc8439)

---

**Last Updated**: 2026-01-17
**Version**: 0.1.0
