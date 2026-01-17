# Create the project structure (local commands)

You will scaffold the base Tauri + Svelte app, then overwrite files with the ones from this draft.

## Option A: Recommended scaffold (create-tauri-app)

From the parent directory where you want the project folder:

1) Create a new project:

- npm:
  - `npm create tauri-app@latest the-organizer`

2) When the interactive prompt asks:
- Select a Svelte + TypeScript frontend template.
- Select Tauri v2 (stable).

3) Enter the project:
- `cd the-organizer`

4) Overwrite the generated files with the drafted files from this chat (matching paths).

## Option B: Manual skeleton (if you do not want the generator)

From an empty directory named `the-organizer`, create this structure:

.
├── package.json
├── src/
│   ├── main.ts
│   ├── App.svelte
│   ├── app.css
│   ├── lib/
│   │   ├── api.ts
│   │   └── stores.ts
│   └── components/
│       ├── Login.svelte
│       ├── Setup.svelte
│       ├── Dashboard.svelte
│       └── EntryModal.svelte
└── src-tauri/
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    └── src/
        ├── main.rs
        ├── lib.rs
        ├── models.rs
        ├── vault.rs
        └── commands.rs

Create directories with:
- macOS/Linux:
  - `mkdir -p src/lib src/components src-tauri/src`
- Windows PowerShell:
  - `mkdir src,src\lib,src\components,src-tauri,src-tauri\src`

Then add files by copying the contents from the code blocks in this chat.
