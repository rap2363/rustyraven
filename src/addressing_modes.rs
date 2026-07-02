use crate::cpu::Cpu;

pub enum AddressingMode {
    Immediate(u8),          // (data)
    Absolute(u16),          // *($HHLL)
    ZeroPage(u8),           // *($00LL)
    IndexedX(u16),          // *($HHLL + X)
    IndexedY(u16),          // *($HHLL + Y)
    IndexedZeroPageX(u8),   // *($00LL + X)
    IndexedZeroPageY(u8),   // *($00LL + Y)
    Indirect(u16),          // **($HHLL)
    IndirectZeroPageX(u8),  // **($00LL + X)
    IndirectZeroPageY(u8),  // *(*($00LL) + Y)
    Relative(u8),           // (data to be used as an offset for branches)
}

#[derive(Debug, PartialEq)]
pub enum PageBoundaryResult {
    Irrelevant,
    SamePage,
    PageBoundaryCrossed,
}

impl AddressingMode {
    // Each addressing mode ultimately returns a byte.
    pub fn into_data(self, cpu: &Cpu) -> (u8, PageBoundaryResult) {
        match self {
            Self::Immediate(d) => (d, PageBoundaryResult::Irrelevant),
            Self::Absolute(address) => (cpu.memory.get_byte(address), PageBoundaryResult::Irrelevant),
            Self::ZeroPage(address) => (cpu.memory.get_byte_zero_page(address), PageBoundaryResult::Irrelevant),
            Self::IndexedX(address) => {
                let pbr = if (address as u8).overflowing_add(cpu.x).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                (cpu.memory.get_byte(address.wrapping_add(cpu.x as u16)), pbr)
            },
            Self::IndexedY(address) => {
                let pbr = if (address as u8).overflowing_add(cpu.y).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                (cpu.memory.get_byte(address.wrapping_add(cpu.y as u16)), pbr)
            },
            Self::IndexedZeroPageX(address) => {
                (cpu.memory.get_byte_zero_page(address.wrapping_add(cpu.x)), PageBoundaryResult::Irrelevant)
            },
            Self::IndexedZeroPageY(address) => {
                (cpu.memory.get_byte_zero_page(address.wrapping_add(cpu.y)), PageBoundaryResult::Irrelevant)
            },
            Self::Indirect(address) => {
                let ptr_address = cpu.memory.get_two_bytes(address);
                (cpu.memory.get_byte(ptr_address), PageBoundaryResult::Irrelevant)
            },
            Self::IndirectZeroPageX(address) => {
                let ptr_address = cpu.memory.get_two_bytes_zero_page(address.wrapping_add(cpu.x));
                (cpu.memory.get_byte(ptr_address), PageBoundaryResult::Irrelevant)
            },
            Self::IndirectZeroPageY(address) => {
                let ptr_address = cpu.memory.get_two_bytes_zero_page(address);
                let pbr = if (ptr_address as u8).overflowing_add(cpu.y).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                let ptr_address = ptr_address.wrapping_add(cpu.y as u16);
                (cpu.memory.get_byte(ptr_address), pbr)
            },
            Self::Relative(offset) => {
                // Check if PC + offset would result in an overflow
                let pbr = if (cpu.pc as u8).overflowing_add(offset).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                (offset, pbr)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_immediate() {
        assert_eq!((0x42, PageBoundaryResult::Irrelevant), AddressingMode::Immediate(0x42).into_data(&Cpu::initialize()));
    }

    #[test]
    fn test_absolute() {
        let mut cpu = Cpu::initialize();
        cpu.memory.set_byte(0x1234, 0x42);
        assert_eq!((0x42, PageBoundaryResult::Irrelevant), AddressingMode::Absolute(0x1234).into_data(&cpu));
    }

    #[test]
    fn test_zero_page() {
        let mut cpu = Cpu::initialize();
        cpu.memory.set_byte(0x0034, 0x42);
        assert_eq!((0x42, PageBoundaryResult::Irrelevant), AddressingMode::ZeroPage(0x34).into_data(&cpu));
    }

    #[test]
    fn test_indexed_x() {
        let mut cpu = Cpu::initialize();
        cpu.x = 0x34;
        cpu.memory.set_byte(0x1234, 0x42);
        assert_eq!((0x42, PageBoundaryResult::SamePage), AddressingMode::IndexedX(0x1200).into_data(&cpu));

        cpu.memory.set_byte(0x1333, 0x43);
        assert_eq!((0x43, PageBoundaryResult::PageBoundaryCrossed), AddressingMode::IndexedX(0x12FF).into_data(&cpu));
    }

    #[test]
    fn test_indexed_y() {
        let mut cpu = Cpu::initialize();
        cpu.y = 0x34;
        cpu.memory.set_byte(0x1234, 0x42);
        assert_eq!((0x42, PageBoundaryResult::SamePage), AddressingMode::IndexedY(0x1200).into_data(&cpu));

        cpu.memory.set_byte(0x1333, 0x43);
        assert_eq!((0x43, PageBoundaryResult::PageBoundaryCrossed), AddressingMode::IndexedY(0x12FF).into_data(&cpu));
    }

    #[test]
    fn test_indexed_zero_page_x() {
        let mut cpu = Cpu::initialize();
        cpu.x = 0x35;
        cpu.memory.set_byte(0x0034, 0x42);
        assert_eq!((0x42, PageBoundaryResult::Irrelevant), AddressingMode::IndexedZeroPageX(0xFF).into_data(&cpu));
    }

    #[test]
    fn test_indexed_zero_page_y() {
        let mut cpu = Cpu::initialize();
        cpu.y = 0x35;
        cpu.memory.set_byte(0x0034, 0x42);
        assert_eq!((0x42, PageBoundaryResult::Irrelevant), AddressingMode::IndexedZeroPageY(0xFF).into_data(&cpu));
    }

    #[test]
    fn test_indirect() {
        let mut cpu = Cpu::initialize();
        cpu.memory.set_byte(0xFFFF, 0x42);
        cpu.memory.set_byte(0x0000, 0x43);
        cpu.memory.set_byte(0x4342, 0x42);
        assert_eq!((0x42, PageBoundaryResult::Irrelevant), AddressingMode::Indirect(0xFFFF).into_data(&cpu));
    }

    #[test]
    fn test_indirect_zero_page_x() {
        let mut cpu = Cpu::initialize();
        cpu.x = 0x0F;
        cpu.memory.set_byte(0x00FF, 0x42);
        cpu.memory.set_byte(0x0000, 0x43);
        cpu.memory.set_byte(0x4342, 0x42);
        assert_eq!((0x42, PageBoundaryResult::Irrelevant), AddressingMode::IndirectZeroPageX(0xF0).into_data(&cpu));
    }

    #[test]
    fn test_indirect_zero_page_y() {
        let mut cpu = Cpu::initialize();
        cpu.y = 0xCF;
        cpu.memory.set_byte(0x00FF, 0x40);
        cpu.memory.set_byte(0x0000, 0x43);
        cpu.memory.set_byte(0x440F, 0x42);
        assert_eq!((0x42, PageBoundaryResult::PageBoundaryCrossed), AddressingMode::IndirectZeroPageY(0xFF).into_data(&cpu));
    }


    #[test]
    fn test_relative() {
        let mut cpu = Cpu::initialize();
        cpu.pc = 0x1234;
        assert_eq!((0x22, PageBoundaryResult::SamePage), AddressingMode::Relative(0x22).into_data(&cpu));
        assert_eq!((0xFE, PageBoundaryResult::PageBoundaryCrossed), AddressingMode::Relative(0xFE).into_data(&cpu));
    }
}