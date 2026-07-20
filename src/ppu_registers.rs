// Byte layout:
// VPHB SINN
// V: Whether NMI is enabled (the vblank nmi enabled)
// P: Master/slave mode (ignored)
// H: Sprite size (0 => 8x8, 1 => 8x16)
// B: Background Pattern table address (0 => 0x0000, 1 => 0x1000)
// S: Sprite Pattern Table Address (0 => 0x0000, 1 => 0x1000 (ignored if 8x16))
// I: VRAM address increment when CPU reads/writes to PPU_DATA (0: add 1, going across; 1: add 32, going down)
// NN: Base Nametable address (0 => 0x2000; 1 => 0x2400; 2 => 0x2800; 3 => 0x2C00)
#[derive(Copy, Clone, Debug)]
pub struct PpuControl(u8);

#[derive(Debug, PartialEq)]
enum SpriteSize {
    EightByEight,
    EightBySixteen,
}

impl PpuControl {
    pub fn from(byte: u8) -> Self {
        Self(byte)
    }

    pub fn is_nmi(self) -> bool {
        (self.0 >> 7 & 0x01) == 0x01
    }

    pub fn sprite_size(self) -> SpriteSize {
        if (self.0 >> 5 & 0x01) == 0x00 { SpriteSize::EightByEight } else { SpriteSize::EightBySixteen }
    }

    pub fn bg_pattern_table_address(self) -> u16 {
        ((self.0 >> 4 & 0x01) << 3) as u16
    }

    pub fn sprite_pattern_table_address(self) -> u16 {
        (self.0 & 0x08) as u16
    }

    pub fn vram_address_increment(self) -> u16 {
        if (self.0 >> 2 & 0x01) == 0x00 { 1 } else { 32 }
    }

    pub fn base_name_table_address(self) -> u16 {
        let nn = self.0 & 0x03;
        match nn {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => panic!("Impossible pattern found {} from {}", nn, self.0),
        }
    }
}

// Byte layout:
// BGRs bMmG
// BGR: Color emphasis bit flags
// s: Enable sprite
// b: Enable background
// M: Sprite left column enable
// m: Background left column enable
// G: Greyscale
#[derive(Copy, Clone, Debug)]
pub struct PpuMask(u8);

impl PpuMask {
    pub fn from(byte: u8) -> Self {
        Self(byte)
    }

    pub fn sprites_enabled(self) -> bool {
        (self.0 >> 4 & 0x01) == 0x01
    }

    pub fn bg_enabled(self) -> bool {
        (self.0 >> 3 & 0x01) == 0x01
    }
}

// Byte layout:
// VSO- ----
// V: vblank
// S: Sprite 0 hit
// O: Sprite Overflow
#[derive(Copy, Clone, Debug)]
pub struct PpuStatus(u8);

impl PpuStatus {
    pub fn from(byte: u8) -> Self {
        Self(byte)
    }

    pub fn is_vblank(self) -> bool {
        (self.0 >> 7 & 0x01) == 0x01
    }

    pub fn sprite_zero_hit(self) -> bool {
        (self.0 >> 6 & 0x01) == 0x01
    }

    pub fn sprite_overflow(self) -> bool {
        (self.0 >> 5 & 0x01) == 0x01
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ppu_control() {
        let ppu_control = PpuControl(0x82);
        assert_eq!(ppu_control.base_name_table_address(), 0x2800);
        assert_eq!(ppu_control.sprite_size(), SpriteSize::EightByEight);
        assert_eq!(ppu_control.vram_address_increment(), 1);
        assert!(ppu_control.is_nmi());
    }
}