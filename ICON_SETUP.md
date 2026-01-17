# Icon Setup Instructions

## Issue

Tauri requires application icons for building on Windows. The build process failed with:

```
`icons/icon.ico` not found; required for generating a Windows Resource file during tauri-build
```

## Quick Fix (Recommended)

Use Tauri's built-in icon generator to create all required icon formats from a single source image:

### Option 1: Generate Icons from Tauri CLI

```bash
# Install the Tauri CLI if not already installed
npm install -D @tauri-apps/cli

# Generate icons from a source PNG (1024x1024 or 512x512 recommended)
# This will create all required icon files in src-tauri/icons/
npx tauri icon path/to/your-icon.png
```

If you don't have a source icon, you can create a simple placeholder:

### Option 2: Create a Simple Placeholder Icon

1. **Create a simple 512x512 PNG image** using any image editor (Paint, GIMP, Photoshop, etc.)
   - Solid color background
   - Simple text "TO" (for "The Organizer")
   - Save as `app-icon.png` in the project root

2. **Generate icons from it**:
   ```bash
   npx tauri icon app-icon.png
   ```

### Option 3: Use Online Icon Generator

1. Visit https://icon.kitchen/ or similar online icon generator
2. Create a simple icon design
3. Download the icon pack
4. Extract to `src-tauri/icons/` directory

## Required Icon Files

The following icon files are needed in `src-tatauri/icons/`:

- `32x32.png` - 32x32 pixels
- `128x128.png` - 128x128 pixels
- `128x128@2x.png` - 256x256 pixels (high DPI)
- `icon.icns` - macOS icon bundle
- `icon.ico` - Windows icon file (multi-resolution)

## After Creating Icons

Once icons are created, run the build again:

```bash
cd src-tauri
cargo build
```

## Note

Icons are only required for building the application bundle. For development purposes, you can use simple placeholder icons generated with the Tauri CLI.
