# The Organizer 🔐

A **local, zero-knowledge encrypted password manager** built as a secure desktop application.

**Version**: 0.1.0
**License**: MIT (see [LICENSE](LICENSE))
**Status**: Early-access for personal use (single-device, offline-first)

---

## Features

- ✅ **Zero-knowledge encryption** - Master password never leaves your device
- ✅ **Strong cryptography** - Argon2id (KDF) + XChaCha20-Poly1305 (AEAD)
- ✅ **Auto-lock** - Automatically locks after 5 minutes of inactivity
- ✅ **Clipboard security** - Passwords auto-clear from clipboard after 15 seconds
- ✅ **Rate limiting** - Brute force protection with cooldown after failed attempts
- ✅ **Memory safety** - Sensitive data zeroized from memory when locked
- ✅ **Cross-platform** - Windows, macOS, Linux support
- ✅ **Offline-first** - No network access, all data stored locally
- ✅ **Master password rotation** - Change master password without recreating the vault
- ✅ **Encrypted backups** - Import/export vault backups using your master password

---

## Technology Stack

**Backend**:
- **Rust** - Memory-safe systems programming language
- **Tauri v2** - Lightweight desktop app framework (vs Electron)
- **Argon2id** - Memory-hard key derivation function (resists GPU attacks)
- **XChaCha20-Poly1305** - Authenticated encryption with extended nonce

**Frontend**:
- **Svelte 5** - Reactive UI framework
- **TypeScript** - Type-safe JavaScript
- **Tailwind CSS** - Utility-first styling

**Security**:
- Encryption/decryption handled in the Rust backend via Tauri IPC
- Passwords are never returned to the frontend (only provided when creating/unlocking)
- Master password is used only for key derivation and never stored
- Keys and plaintext are zeroized when the vault is locked (best-effort)
- Vault format versioning for future-proofing

---

## Quick Start

### Prerequisites

- **Node.js** 18+ and npm
- **Rust** 1.70+ and Cargo
- **OS**: Windows 10+, macOS 10.15+, or Linux (any modern distro)

### Installation

1. **Clone the repository**:
   ```bash
   git clone https://github.com/wojmat/the-organizer.git
   cd the-organizer
   ```

2. **Install frontend dependencies**:
   ```bash
   npm install
   ```

3. **Run the development build**:
   ```bash
   npm run tauri:dev
   ```

   This will:
   - Start Vite dev server (http://localhost:5173)
   - Compile Rust backend
   - Launch the desktop app

### Build for Production

```bash
npm run tauri:build
```

Installers will be generated in `src-tauri/target/release/bundle/`:
- **Windows**: `.msi` or `.exe`
- **macOS**: `.dmg` or `.app`
- **Linux**: `.deb`, `.AppImage`, or `.rpm`

---

## Usage Guide

### 1. First Run - Create Vault

When you first launch the app, you'll see the **Setup** screen:

1. Enter a strong master password (minimum 10 characters enforced in the UI)
2. Confirm the password
3. Click "Create Vault"

**Important**: The master password is never stored. If you forget it, your vault cannot be recovered.

### 2. Unlock Vault

On subsequent launches, enter your master password on the **Login** screen.

**Security**: After 5 failed unlock attempts, the app enforces a 30-second cooldown.

### 3. Add Password Entries

Once unlocked, you'll see the **Dashboard**:

1. Click the "Add" button
2. Fill in the entry details:
   - **Title**: Name of the entry (required)
   - **Username**: Email or username
   - **Password**: The password to store (required)
   - **URL**: Website URL (optional)
   - **Notes**: Additional information (optional)
3. Click "Save"

### 4. Copy Passwords

To use a stored password:

1. Find the entry in your list
2. Click the "Copy" button
3. Paste the password where needed

**Security**: The clipboard automatically clears after 15 seconds.

### 5. Delete Entries

1. Click "Delete" on any entry
2. Confirm the deletion (this action cannot be undone)

### 6. Rotate Master Password

1. Open the backup/maintenance panel in the dashboard
2. Enter your current master password
3. Choose a new master password and confirm
4. Save to re-encrypt the vault with a fresh salt and key

### 7. Import/Export Encrypted Backups

1. In the dashboard, enter a file path for export
2. Click **Export** to write an encrypted backup
3. For import, provide the backup path and master password used for that backup
4. Click **Import** to replace the local vault with the backup contents

### 8. Lock the Vault

- Click the "Lock" button in the header to manually lock
- The vault also auto-locks after 5 minutes of inactivity

---

## Data Storage

**Vault Location** (Tauri app data directory):
- **Windows**: `%APPDATA%\com.theorganizer.app\vault.dat`
- **macOS**: `~/Library/Application Support/com.theorganizer.app/vault.dat`
- **Linux**: `~/.local/share/com.theorganizer.app/vault.dat`

Paths can vary by OS configuration. The app resolves the data directory at runtime.

**Vault File Format**:
```
[1 byte version][32 bytes salt][24 bytes nonce][encrypted data + auth tag]
```

**What's encrypted**: All entry data (titles, usernames, passwords, URLs, notes)
**What's NOT encrypted**: Salt, nonce, version byte (safe to expose)

---

## Security

### What We Protect Against ✅

- **Vault file theft** - Strong encryption renders stolen vault useless without password
- **Memory dumps** - Sensitive data zeroized when locked
- **Brute force attacks** - Argon2id makes password guessing computationally expensive
- **Replay attacks** - Fresh nonce per save prevents ciphertext reuse

### What We DON'T Protect Against ❌

- **Keyloggers** - Malicious software that records keystrokes
- **Screen recording** - Malware capturing screen contents
- **Compromised OS** - Root-level malware with full system access
- **Physical access** - Attacker with access to unlocked app

### Cryptographic Details

**Key Derivation (Argon2id)**:
- Memory cost: 64 MiB
- Time cost: 3 iterations
- Parallelism: 1 thread
- Output: 32 bytes (256-bit key)

**Encryption (XChaCha20-Poly1305)**:
- Cipher: XChaCha20 (ChaCha20 with extended nonce)
- Authentication: Poly1305 MAC (prevents tampering)
- Nonce: 24 bytes (randomly generated per save)

For more details, see [docs/SECURITY.md](docs/SECURITY.md).

---

## Development

### Project Structure

```
the-organizer/
├── src/                    # Frontend (Svelte + TypeScript)
│   ├── App.svelte         # Root component
│   ├── main.ts            # Entry point
│   ├── lib/
│   │   ├── api.ts         # Tauri IPC bindings
│   │   └── stores.ts      # Svelte stores
│   └── components/
│       ├── Setup.svelte   # Vault creation
│       ├── Login.svelte   # Unlock screen
│       ├── Dashboard.svelte
│       └── EntryModal.svelte
├── src-tauri/              # Backend (Rust + Tauri)
│   ├── src/
│   │   ├── main.rs        # App entry + auto-lock
│   │   ├── lib.rs         # Module root
│   │   ├── commands.rs    # Tauri command handlers
│   │   ├── vault.rs       # Encryption/decryption
│   │   └── models.rs      # Data structures
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── docs/                   # Documentation
│   ├── ARCHITECTURE.md    # System design
│   └── SECURITY.md        # Security model
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md              # This file
```

### Running Tests

**Rust backend tests**:
```bash
cd src-tauri
cargo test
```

**TypeScript type checking**:
```bash
npm run check
```

**Linting**:
```bash
cd src-tauri
cargo clippy
```

### Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests and type checks
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

**Security**: If you discover a security vulnerability, please email [security@example.com] (replace with a real contact) instead of opening a public issue.

---

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed system design documentation.

**High-level flow**:
```
User Input → Svelte UI → Tauri IPC → Rust Backend → Vault File
                                    ↓
                             Argon2id + XChaCha20
                                    ↓
                            Encrypted vault.dat
```

---

## Roadmap

- [x] Core password manager functionality
- [x] Auto-lock on inactivity
- [x] Clipboard auto-clear
- [x] Rate limiting on unlock attempts
- [x] Vault format versioning
- [x] Master password change
- [x] Import/export (encrypted backup)
- [ ] Password generator
- [ ] Password strength meter
- [ ] Audit logging
- [ ] Browser extension integration

---

## Troubleshooting

**Issue**: App won't start / blank window
**Solution**: Check console for errors (Ctrl+Shift+I). Ensure Node.js and Rust are installed.

**Issue**: `npm install` fails
**Solution**: Clear cache with `npm cache clean --force`, then retry.

**Issue**: Rust compilation fails with "linker not found"
**Solution**:
- Windows: Install Visual Studio Build Tools with C++ workload
- Linux: Install build-essential (`sudo apt install build-essential`)

**Issue**: Forgot master password
**Solution**: Unfortunately, the vault cannot be recovered. Create a new vault with a new password.

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- **Tauri** - For making lightweight desktop apps possible
- **Argon2** - Winner of the Password Hashing Competition
- **XChaCha20-Poly1305** - Modern authenticated encryption

---

**Built with security and privacy in mind** 🔒
