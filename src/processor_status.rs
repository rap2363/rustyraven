// Holds the statuses of flags. From high to low:
// N | V |   | B | D | I | Z | C
#[derive(Clone, Copy, Debug)]
pub struct ProcessorStatus(u8);

impl ProcessorStatus {
    pub fn initialize() -> Self {
        Self(0x00)
    }

    pub fn into(self) -> u8 {
        self.0
    }

    pub fn from(flags: u8) -> Self {
        Self(flags)
    }

    // Carry Flag
    pub fn set_carry(self) -> Self {
        Self::from(self.0 | 0x01)
    }

    pub fn clear_carry(self) -> Self {
        Self::from(self.0 & (0xFF - 0x01))
    }

    pub fn is_carry(&self) -> bool {
        self.carry() == 0x01
    }

    pub fn carry(&self) -> u8 {
        self.0 & 0x01
    }

    // Zero Flag
    pub fn set_zero(self) -> Self {
        Self::from(self.0 | 0x02)
    }

    pub fn clear_zero(self) -> Self {
        Self::from(self.0 & (0xFF - 0x02))
    }

    pub fn is_zero(&self) -> bool {
        (self.0 >> 1) & 0x01 == 0x01
    }

    // Interrupt Flag
    pub fn set_interrupt(self) -> Self {
        Self::from(self.0 | 0x04)
    }

    pub fn clear_interrupt(self) -> Self {
        Self::from(self.0 & (0xFF - 0x04))
    }

    pub fn is_interrupt(&self) -> bool {
        (self.0 >> 2) & 0x01 == 0x01
    }

    // Break Flag
    pub fn set_break(self) -> Self {
        Self::from(self.0 | 0x10)
    }

    pub fn clear_break(self) -> Self {
        Self::from(self.0 & (0xFF - 0x10))
    }

    pub fn is_break(&self) -> bool {
        (self.0 >> 4) & 0x01 == 0x01
    }

    // Overflow Flag
    pub fn set_overflow(self) -> Self {
        Self::from(self.0 | 0x40)
    }

    pub fn clear_overflow(self) -> Self {
        Self::from(self.0 & (0xFF - 0x40))
    }

    pub fn is_overflow(&self) -> bool {
        (self.0 >> 6) & 0x01 == 0x01
    }

    // Negative Flag
    pub fn set_negative(self) -> Self {
        Self::from(self.0 | 0x80)
    }

    pub fn clear_negative(self) -> Self {
        Self::from(self.0 & (0xFF - 0x80))
    }

    pub fn is_negative(&self) -> bool {
        (self.0 >> 7) & 0x01 == 0x01
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_statuses() {
        let ps = ProcessorStatus::initialize().set_interrupt().set_break().set_negative().set_carry();

        assert!(ps.is_break());
        assert!(ps.is_interrupt());
        assert!(ps.is_negative());
        assert!(ps.is_carry());

        assert!(!ps.is_zero());
        assert!(!ps.is_overflow());
    }
}