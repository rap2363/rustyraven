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

pub struct AddressingModeData {
    pub data: u8, // The actual data picked up by this addressing mode
    pub address: Option<u16>, // The address of the data. Might be None for implied data.
    pub page_boundary_result: PageBoundaryResult,
}

impl AddressingModeData {
    fn new(data: u8, address: Option<u16>, page_boundary_result: PageBoundaryResult) -> Self {
        Self { data, address, page_boundary_result }
    }
}

impl AddressingMode {
    // Each addressing mode returns data, the address of said data (if possible), and whether or not a page boundary
    // was crossed. This is all measured with the PC set to the *next* instruction.
    pub fn into_data(self, cpu: &Cpu) -> AddressingModeData {
        match self {
            Self::Implied => AddressingModeData::new(0x00, None, PageBoundaryResult::Irrelevant),
            Self::Immediate(d) => AddressingModeData::new(d, None, PageBoundaryResult::Irrelevant),
            Self::Absolute(address) => AddressingModeData::new(cpu.memory.read_byte(address), Some(address), PageBoundaryResult::Irrelevant),
            Self::ZeroPage(address) => AddressingModeData::new(cpu.memory.read_byte_zero_page(address), Some(address as u16), PageBoundaryResult::Irrelevant),
            Self::IndexedX(address) => {
                let pbr = if (address as u8).overflowing_add(cpu.x).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                let final_address = address.wrapping_add(cpu.x as u16);
                AddressingModeData::new(cpu.memory.read_byte(final_address), Some(final_address), pbr)
            },
            Self::IndexedY(address) => {
                let pbr = if (address as u8).overflowing_add(cpu.y).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                
                let final_address = address.wrapping_add(cpu.y as u16);
                AddressingModeData::new(cpu.memory.read_byte(final_address), Some(final_address), pbr)
            },
            Self::IndexedZeroPageX(address) => {
                let final_address = address.wrapping_add(cpu.x);
                AddressingModeData::new(cpu.memory.read_byte_zero_page(final_address), Some(final_address as u16), PageBoundaryResult::Irrelevant)
            },
            Self::IndexedZeroPageY(address) => {
                let final_address = address.wrapping_add(cpu.y);
                AddressingModeData::new(cpu.memory.read_byte_zero_page(final_address), Some(final_address as u16), PageBoundaryResult::Irrelevant)
            },
            Self::Indirect(address) => {
                let ptr_address = cpu.memory.read_two_bytes_wrapping_page(address);
                AddressingModeData::new(0x00, Some(ptr_address), PageBoundaryResult::Irrelevant)
            },
            Self::IndirectZeroPageX(address) => {
                let ptr_address = cpu.memory.read_two_bytes_zero_page(address.wrapping_add(cpu.x));
                AddressingModeData::new(cpu.memory.read_byte(ptr_address), Some(ptr_address), PageBoundaryResult::Irrelevant)
            },
            Self::IndirectZeroPageY(address) => {
                let ptr_address = cpu.memory.read_two_bytes_zero_page(address);
                let pbr = if (ptr_address as u8).overflowing_add(cpu.y).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                let ptr_address = ptr_address.wrapping_add(cpu.y as u16);
                AddressingModeData::new(cpu.memory.read_byte(ptr_address), Some(ptr_address), pbr)
            },
            Self::Relative(offset) => {
                // Check if PC + offset would result in an overflow
                let final_address = cpu.pc as u16 + offset as u16;
                let pbr = if (cpu.pc as u8).overflowing_add(offset).1 {
                    PageBoundaryResult::PageBoundaryCrossed
                } else {
                    PageBoundaryResult::SamePage
                };
                AddressingModeData::new(offset, Some(final_address), pbr)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implied() {
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::Implied.into_data(&Cpu::initialize());
        assert_eq!(0x00, data);
        assert_eq!(None, address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_immediate() {
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::Immediate(0x42).into_data(&Cpu::initialize());
        assert_eq!(0x42, data);
        assert_eq!(None, address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_absolute() {
        let mut cpu = Cpu::initialize();
        cpu.memory.write_byte(0x1234, 0x42);
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::Absolute(0x1234).into_data(&cpu);
        assert_eq!(0x42, data);
        assert_eq!(Some(0x1234), address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_zero_page() {
        let mut cpu = Cpu::initialize();
        cpu.memory.write_byte(0x0034, 0x42);
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::ZeroPage(0x34).into_data(&cpu);
        assert_eq!(0x42, data);
        assert_eq!(Some(0x0034), address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_indexed_x() {
        let mut cpu = Cpu::initialize();
        cpu.x = 0x34;
        cpu.memory.write_byte(0x1234, 0x42);
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::IndexedX(0x1200).into_data(&cpu);
        assert_eq!(0x42, data);
        assert_eq!(Some(0x1234), address);
        assert_eq!(PageBoundaryResult::SamePage, page_boundary_result);

        cpu.memory.write_byte(0x1333, 0x43);
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::IndexedX(0x12FF).into_data(&cpu);
        assert_eq!(0x43, data);
        assert_eq!(Some(0x1333), address);
        assert_eq!(PageBoundaryResult::PageBoundaryCrossed, page_boundary_result);
    }

    #[test]
    fn test_indexed_y() {
        let mut cpu = Cpu::initialize();
        cpu.y = 0x34;
        cpu.memory.write_byte(0x1234, 0x42);
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::IndexedY(0x1200).into_data(&cpu);
        assert_eq!(0x42, data);
        assert_eq!(Some(0x1234), address);
        assert_eq!(PageBoundaryResult::SamePage, page_boundary_result);

        cpu.memory.write_byte(0x1333, 0x43);
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::IndexedY(0x12FF).into_data(&cpu);
        assert_eq!(0x43, data);
        assert_eq!(Some(0x1333), address);
        assert_eq!(PageBoundaryResult::PageBoundaryCrossed, page_boundary_result);
    }

    #[test]
    fn test_indexed_zero_page_x() {
        let mut cpu = Cpu::initialize();
        cpu.x = 0x35;
        cpu.memory.write_byte(0x0034, 0x42);
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::IndexedZeroPageX(0xFF).into_data(&cpu);
        assert_eq!(0x42, data);
        assert_eq!(Some(0x0034), address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_indexed_zero_page_y() {
        let mut cpu = Cpu::initialize();
        cpu.y = 0x35;
        cpu.memory.write_byte(0x0034, 0x42);
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::IndexedZeroPageY(0xFF).into_data(&cpu);
        assert_eq!(0x42, data);
        assert_eq!(Some(0x0034), address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_indirect() {
        let mut cpu = Cpu::initialize();
        cpu.memory.write_byte(0xFFFF, 0x42);
        cpu.memory.write_byte(0x0000, 0x43);
        cpu.memory.write_byte(0x4342, 0x42);
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::Indirect(0xFFFF).into_data(&cpu);
        assert_eq!(0x42, data);
        assert_eq!(Some(0x4342), address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_indirect_zero_page_x() {
        let mut cpu = Cpu::initialize();
        cpu.x = 0x0F;
        cpu.memory.write_byte(0x00FF, 0x42);
        cpu.memory.write_byte(0x0000, 0x43);
        cpu.memory.write_byte(0x4342, 0x42);
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::IndirectZeroPageX(0xF0).into_data(&cpu);
        assert_eq!(0x42, data);
        assert_eq!(Some(0x4342), address);
        assert_eq!(PageBoundaryResult::Irrelevant, page_boundary_result);
    }

    #[test]
    fn test_indirect_zero_page_y() {
        let mut cpu = Cpu::initialize();
        cpu.y = 0xCF;
        cpu.memory.write_byte(0x00FF, 0x40);
        cpu.memory.write_byte(0x0000, 0x43);
        cpu.memory.write_byte(0x440F, 0x42);
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::IndirectZeroPageY(0xFF).into_data(&cpu);
        assert_eq!(0x42, data);
        assert_eq!(Some(0x440F), address);
        assert_eq!(PageBoundaryResult::PageBoundaryCrossed, page_boundary_result);
    }


    #[test]
    fn test_relative() {
        let mut cpu = Cpu::initialize();
        cpu.pc = 0x1234;
        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::Relative(0x22).into_data(&cpu);
        assert_eq!(0x22, data);
        assert_eq!(Some(0x1256), address);
        assert_eq!(PageBoundaryResult::SamePage, page_boundary_result);

        let AddressingModeData { data, address, page_boundary_result } = AddressingMode::Relative(0xFE).into_data(&cpu);
        assert_eq!(0xFE, data);
        assert_eq!(Some(0x1332), address);
        assert_eq!(PageBoundaryResult::PageBoundaryCrossed, page_boundary_result);
    }
}