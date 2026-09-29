The prospector icon was generated with the built-in imagegen tool for this native
port. It is new artwork, not an extracted original game asset.

`ldm-icon-master.png` is the generated transparent master. `ldm.ico` contains
256, 128, 64, 48, 32, 24 and 16 pixel variants for the Windows executable and
shortcuts. `ldm-icon.bmp` is the 64 pixel version loaded by SDL on every platform.
These exports were made with ImageMagick:

```sh
magick resources/ldm-icon-master.png -define icon:auto-resize=256,128,64,48,32,24,16 resources/ldm.ico
magick resources/ldm-icon-master.png -resize 64x64 -define bmp:format=bmp4 resources/ldm-icon.bmp
```

The display settings menu uses `ui-font.bmp`, a pre-baked DejaVu Sans ASCII
atlas built into the executable. `crates/desktop/src/font/glyphs.rs` holds its
glyph metrics, and `ui-font.json` records the source font hash. Regenerate them
with `python3 tools/bake_font.py /path/to/DejaVuSans.ttf` (Pillow and
ImageMagick are build tools only). The font's license is in `FONT-LICENSE.txt`
and is included in playable bundles. No system font is required at runtime.

Generation prompt:

> Create one finished square desktop application icon for a faithful native port of the 1989 adventure game Lost Dutchman Mine. Asset type: Windows .ico / Linux application icon, master raster image. Style: authentic chunky late-1980s VGA pixel art, bold crisp pixel clusters, limited warm desert palette. Subject: a friendly rugged old gold prospector shown as a strong chest-up silhouette, broad tan cowboy hat, prominent white beard, red neckerchief and denim blue shoulders, with a single gold pickaxe visible over one shoulder. Strong dark brown outline, warm gold highlights, uncluttered and instantly legible at 32 by 32 pixels. Center the subject and leave a little clear margin; one icon only, no surrounding mockup. Background must be genuinely transparent alpha, including corners, no checkerboard, no filled square. No text, letters, logo words, watermarks, gradients, glossy modern effects or photorealism. Square 1024 by 1024 master.
