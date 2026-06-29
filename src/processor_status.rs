// Holds the statuses of flags. From high to low:
// N | V |   | B | D | I | Z | C
#[derive(Debug)]
pub struct ProcessorStatus {
    flags: u8,
}

impl ProcessorStatus {
    pub fn initialize() -> Self {
        Self {
            flags: 0x00,
        }
    }

    fn from(flags: u8) -> Self {
        Self { flags }
    }

    // Carry Flag
    pub fn set_carry(self) -> Self {
        Self::from(self.flags | 0x01)
    }

    pub fn clear_carry(self) -> Self {
        Self::from(self.flags & (0xFF - 0x01))
    }

    pub fn is_carry(&self) -> bool {
        self.flags & 0x01 == 0x01
    }

    // Zero Flag
    pub fn set_zero(self) -> Self {
        Self::from(self.flags | 0x02)
    }

    pub fn clear_zero(self) -> Self {
        Self::from(self.flags & (0xFF - 0x02))
    }

    pub fn is_zero(&self) -> bool {
        (self.flags >> 1) & 0x01 == 0x01
    }

    // Interrupt Flag
    pub fn set_interrupt(self) -> Self {
        Self::from(self.flags | 0x04)
    }

    pub fn clear_interrupt(self) -> Self {
        Self::from(self.flags & (0xFF - 0x04))
    }

    pub fn is_interrupt(&self) -> bool {
        (self.flags >> 2) & 0x01 == 0x01
    }

    // Break Flag
    pub fn set_break(self) -> Self {
        Self::from(self.flags | 0x10)
    }

    pub fn clear_break(self) -> Self {
        Self::from(self.flags & (0xFF - 0x10))
    }

    pub fn is_break(&self) -> bool {
        (self.flags >> 4) & 0x01 == 0x01
    }

    // Overflow Flag
    pub fn set_overflow(self) -> Self {
        Self::from(self.flags | 0x40)
    }

    pub fn clear_overflow(self) -> Self {
        Self::from(self.flags & (0xFF - 0x40))
    }

    pub fn is_overflow(&self) -> bool {
        (self.flags >> 6) & 0x01 == 0x01
    }

    // Negative Flag
    pub fn set_negative(self) -> Self {
        Self::from(self.flags | 0x80)
    }

    pub fn clear_negative(self) -> Self {
        Self::from(self.flags & (0xFF - 0x80))
    }

    pub fn is_negative(&self) -> bool {
        (self.flags >> 7) & 0x01 == 0x01
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_statuses() {
        let ps = ProcessorStatus::initialize().set_interrupt().set_break().set_negative();

        assert!(ps.is_break());
        assert!(ps.is_interrupt());
        assert!(ps.is_negative());

        assert!(!ps.is_carry());
        assert!(!ps.is_zero());
        assert!(!ps.is_overflow());
    }
}