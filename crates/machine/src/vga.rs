//! The VGA colour registers the BIOS programs.

/// The DAC palette as 0xAARRGGBB colours, and the 16 attribute-controller
/// entries that map text and EGA colours onto it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vga {
    /// DAC palette.
    pub palette: [u32; 256],
    /// Attribute controller palette.
    pub attributes: [u8; 16],
}

impl Default for Vga {
    fn default() -> Self {
        Vga { palette: [0; 256], attributes: [0; 16] }
    }
}
