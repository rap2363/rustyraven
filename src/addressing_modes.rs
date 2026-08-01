use crate::cpu::Cpu;

#[derive(Debug)]
pub enum AddressingMode {
    Implied,                // No data to fill in
    Immediate(u8),          // *($PC)
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AddressingModeData {
    Implied,
    Data(u8), // The actual data (used for implied or immediate modes)
    Address(u16), // The address where the data is stored (used for indirect modes)
}

impl AddressingModeData {
    pub fn as_data(self) -> Option<u8> {
        if let AddressingModeData::Data(data) = self {
            Some(data)
        } else {
            None
        }
    }

    pub fn as_address(self) -> Option<u16> {
        if let AddressingModeData::Address(address) = self {
            Some(address)
        } else {
            None
        }
    }
}

impl AddressingMode {
    // Each addressing mode returns data, the address of said data (if possible), and whether or not a page boundary
    // was crossed. This is all measured with the PC set to the *next* instruction.
    pub fn into_data(self, cpu: &Cpu) -> (AddressingModeData, PageBoundaryResult) {
        match self {
            Self::Implied => (AddressingModeData::Implied, PageBoundaryResult::Irrelevant),
            Self::Immediate(d) => (AddressingModeData::Data(d), PageBoundaryResult::Irrelevant),
            Self::Absolute(address) => (AddressingModeData::Address(address), PageBoundaryResult::Irrelevant),
            Self::ZeroPage(address) => (AddressingModeData::Address(address as u16), PageBoundaryResult::Irrelevant),
            Self::IndexedX(address) => {
                let pbr = if (address as u8).overflowing_add(cpu.x).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                let final_address = address.wrapping_add(cpu.x as u16);
                (AddressingModeData::Address(final_address), pbr)
            },
            Self::IndexedY(address) => {
                let pbr = if (address as u8).overflowing_add(cpu.y).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                
                let final_address = address.wrapping_add(cpu.y as u16);
                (AddressingModeData::Address(final_address), pbr)
            },
            Self::IndexedZeroPageX(address) => {
                let final_address = address.wrapping_add(cpu.x);
                (AddressingModeData::Address(final_address as u16), PageBoundaryResult::Irrelevant)
            },
            Self::IndexedZeroPageY(address) => {
                let final_address = address.wrapping_add(cpu.y);
                (AddressingModeData::Address(final_address as u16), PageBoundaryResult::Irrelevant)
            },
            Self::Indirect(address) => {
                let ptr_address = cpu.memory.read_two_bytes_wrapping_page(address);
                (AddressingModeData::Address(ptr_address), PageBoundaryResult::Irrelevant)
            },
            Self::IndirectZeroPageX(address) => {
                let ptr_address = cpu.memory.read_two_bytes_zero_page(address.wrapping_add(cpu.x));
                (AddressingModeData::Address(ptr_address), PageBoundaryResult::Irrelevant)
            },
            Self::IndirectZeroPageY(address) => {
                let ptr_address = cpu.memory.read_two_bytes_zero_page(address);
                let pbr = if (ptr_address as u8).overflowing_add(cpu.y).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                let ptr_address = ptr_address.wrapping_add(cpu.y as u16);
                (AddressingModeData::Address(ptr_address), pbr)
            },
            Self::Relative(offset) => {
                // Check if PC + offset would result in an overflow
                // let final_address = cpu.pc as u16 + offset as u16;
                let pbr = if (cpu.pc as u8).overflowing_add(offset).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                (AddressingModeData::Data(offset), pbr)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{addressing_modes, rom::NametableArrangement::VerticallyMirrored};

    #[test]
    fn test_implied() {
        let (addressing_mode, page_boundary_result) = AddressingMode::Implied.into_data(&Cpu::initialize(VerticallyMirrored));
        assert_eq!(addressing_mode, AddressingModeData::Implied);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_immediate() {
        let (AddressingModeData::Data(data), page_boundary_result) = AddressingMode::Immediate(0x42).into_data(&Cpu::initialize(VerticallyMirrored)) else { panic!() };
        assert_eq!(0x42, data);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_absolute() {
        let mut cpu = Cpu::initialize(VerticallyMirrored);
        cpu.memory.write_byte(0x1234, 0x42);
        let (AddressingModeData::Address(address), page_boundary_result) = AddressingMode::Absolute(0x1234).into_data(&cpu) else { panic!() };
        assert_eq!(0x1234, address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_zero_page() {
        let mut cpu = Cpu::initialize(VerticallyMirrored);
        cpu.memory.write_byte(0x0034, 0x42);
        let (AddressingModeData::Address(address), page_boundary_result) = AddressingMode::ZeroPage(0x34).into_data(&cpu) else { panic!() };
        assert_eq!(0x0034, address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_indexed_x() {
        let mut cpu = Cpu::initialize(VerticallyMirrored);
        cpu.x = 0x34;
        cpu.memory.write_byte(0x1234, 0x42);
        let (AddressingModeData::Address(address), page_boundary_result) = AddressingMode::IndexedX(0x1200).into_data(&cpu) else { panic!() };
        assert_eq!(0x1234, address);
        assert_eq!(PageBoundaryResult::SamePage, page_boundary_result);

        cpu.memory.write_byte(0x1333, 0x43);
        let (AddressingModeData::Address(address), page_boundary_result) = AddressingMode::IndexedX(0x12FF).into_data(&cpu) else { panic!() };
        assert_eq!(0x1333, address);
        assert_eq!(PageBoundaryResult::PageBoundaryCrossed, page_boundary_result);
    }

    #[test]
    fn test_indexed_y() {
        let mut cpu = Cpu::initialize(VerticallyMirrored);
        cpu.y = 0x34;
        cpu.memory.write_byte(0x1234, 0x42);
        let (AddressingModeData::Address(address), page_boundary_result) = AddressingMode::IndexedY(0x1200).into_data(&cpu) else { panic!() };
        assert_eq!(0x1234, address);
        assert_eq!(PageBoundaryResult::SamePage, page_boundary_result);

        cpu.memory.write_byte(0x1333, 0x43);
        let (AddressingModeData::Address(address), page_boundary_result) = AddressingMode::IndexedY(0x12FF).into_data(&cpu) else { panic!() };
        assert_eq!(0x1333, address);
        assert_eq!(PageBoundaryResult::PageBoundaryCrossed, page_boundary_result);
    }

    #[test]
    fn test_indexed_zero_page_x() {
        let mut cpu = Cpu::initialize(VerticallyMirrored);
        cpu.x = 0x35;
        cpu.memory.write_byte(0x0034, 0x42);
        let (AddressingModeData::Address(address), page_boundary_result) = AddressingMode::IndexedZeroPageX(0xFF).into_data(&cpu) else { panic!() };
        assert_eq!(0x0034, address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_indexed_zero_page_y() {
        let mut cpu = Cpu::initialize(VerticallyMirrored);
        cpu.y = 0x35;
        cpu.memory.write_byte(0x0034, 0x42);
        let (AddressingModeData::Address(address), page_boundary_result) = AddressingMode::IndexedZeroPageY(0xFF).into_data(&cpu) else { panic!() };
        assert_eq!(0x0034, address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_indirect() {
        let mut cpu = Cpu::initialize(VerticallyMirrored);
        cpu.memory.write_byte(0xFFFF, 0x42);
        cpu.memory.write_byte(0xFF00, 0x43);
        let (AddressingModeData::Address(address), page_boundary_result) = AddressingMode::Indirect(0xFFFF).into_data(&cpu) else { panic!() };
        assert_eq!(0x4342, address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_indirect_zero_page_x() {
        let mut cpu = Cpu::initialize(VerticallyMirrored);
        cpu.x = 0x0F;
        cpu.memory.write_byte(0x00FF, 0x42);
        cpu.memory.write_byte(0x0000, 0x43);
        cpu.memory.write_byte(0x4342, 0x42);
        let (AddressingModeData::Address(address), page_boundary_result) = AddressingMode::IndirectZeroPageX(0xF0).into_data(&cpu) else { panic!() };
        assert_eq!(0x4342, address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_indirect_zero_page_y() {
        let mut cpu = Cpu::initialize(VerticallyMirrored);
        cpu.y = 0xCF;
        cpu.memory.write_byte(0x00FF, 0x40);
        cpu.memory.write_byte(0x0000, 0x43);
        cpu.memory.write_byte(0x440F, 0x42);
        let (AddressingModeData::Address(address), page_boundary_result) = AddressingMode::IndirectZeroPageY(0xFF).into_data(&cpu) else { panic!() };
        assert_eq!(0x440F, address);
        assert_eq!(PageBoundaryResult::PageBoundaryCrossed, page_boundary_result);
    }


    #[test]
    fn test_relative() {
        let mut cpu = Cpu::initialize(VerticallyMirrored);
        cpu.pc = 0x1234;
        let (AddressingModeData::Data(data), page_boundary_result) = AddressingMode::Relative(0x22).into_data(&cpu) else { panic!() };
        assert_eq!(0x22, data);
        assert_eq!(PageBoundaryResult::SamePage, page_boundary_result);

        let (AddressingModeData::Data(data), page_boundary_result) = AddressingMode::Relative(0xFE).into_data(&cpu) else { panic!() };
        assert_eq!(0xFE, data);
        assert_eq!(PageBoundaryResult::PageBoundaryCrossed, page_boundary_result);
    }
}