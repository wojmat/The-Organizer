#!/usr/bin/env python3
"""
Generate minimal placeholder icons for The Organizer application.
This creates simple colored icons with the text "TO" for development purposes.
"""

import os
import sys

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    print("Error: Pillow library required. Install with: pip install Pillow")
    sys.exit(1)

def create_icon(size, output_path):
    """Create a simple icon with 'TO' text on a colored background."""
    # Create image with dark blue background
    img = Image.new('RGB', (size, size), color='#1e40af')
    draw = ImageDraw.Draw(img)

    # Try to use a default font, fallback to basic if not available
    try:
        font_size = size // 2
        font = ImageFont.truetype("arial.ttf", font_size)
    except:
        font = ImageFont.load_default()

    # Draw "TO" text in white
    text = "TO"

    # Calculate text position (center)
    bbox = draw.textbbox((0, 0), text, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]
    x = (size - text_width) // 2
    y = (size - text_height) // 2

    draw.text((x, y), text, fill='white', font=font)

    # Save PNG
    img.save(output_path, 'PNG')
    print(f"Created: {output_path}")

    return img

def create_ico(sizes, output_path):
    """Create a Windows ICO file with multiple sizes."""
    images = []
    for size in sizes:
        img = Image.new('RGB', (size, size), color='#1e40af')
        draw = ImageDraw.Draw(img)

        # Simple "TO" text
        try:
            font_size = size // 2
            font = ImageFont.truetype("arial.ttf", font_size)
        except:
            font = ImageFont.load_default()

        text = "TO"
        bbox = draw.textbbox((0, 0), text, font=font)
        text_width = bbox[2] - bbox[0]
        text_height = bbox[3] - bbox[1]
        x = (size - text_width) // 2
        y = (size - text_height) // 2

        draw.text((x, y), text, fill='white', font=font)
        images.append(img)

    # Save as ICO
    images[0].save(output_path, format='ICO', sizes=[(s, s) for s in sizes])
    print(f"Created: {output_path}")

def main():
    # Create icons directory
    icons_dir = os.path.join('src-tauri', 'icons')
    os.makedirs(icons_dir, exist_ok=True)

    print("Generating placeholder icons for The Organizer...")
    print("=" * 60)

    # Create PNG icons
    create_icon(32, os.path.join(icons_dir, '32x32.png'))
    create_icon(128, os.path.join(icons_dir, '128x128.png'))
    create_icon(256, os.path.join(icons_dir, '128x128@2x.png'))

    # Create ICO file for Windows (multiple sizes)
    create_ico([16, 32, 48, 256], os.path.join(icons_dir, 'icon.ico'))

    # Create a basic ICNS placeholder (macOS)
    # Note: Pillow doesn't support ICNS creation, so we create a large PNG
    # Users on macOS will need to convert this properly or use Tauri icon command
    create_icon(512, os.path.join(icons_dir, 'icon.icns.png'))

    print("=" * 60)
    print("\nPlaceholder icons created successfully!")
    print("\nNote: For macOS builds, you'll need to convert icon.icns.png to")
    print("a proper ICNS file using: npx tauri icon src-tauri/icons/icon.icns.png")
    print("\nFor production, replace these with professional icons using:")
    print("  npx tauri icon path/to/your-icon.png")

if __name__ == '__main__':
    main()
