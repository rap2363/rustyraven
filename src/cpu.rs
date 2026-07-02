use std::net::Shutdown::Write;

use crate::addressing_modes::{AddressingMode, AddressingModeData, PageBoundaryResult::PageBoundaryCrossed, WriteLocation};
use crate::memory::CpuMemory;
use crate::processor_status::ProcessorStatus;

#[derive(Debug)]
enum Opcode {
    ADC,
    AND,
    ASL,
}

pub struct Cpu {
    pub memory: CpuMemory,
    pub processor_status: ProcessorStatus,
    pub pc: u16,
    pub sp: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    cycle_count: usize,
}

pub struct FetchInstructionResult {
    opcode: Opcode,
    addressing_mode: AddressingMode,
    cycles: usize,
}

impl FetchInstructionResult {
    fn new(opcode: Opcode, addressing_mode: AddressingMode, cycles: usize) -> Self {
        Self { opcode, addressing_mode, cycles }
    }
}

impl Cpu {
    pub fn initialize() -> Self {
        Self {
            memory: CpuMemory::initialize(),
            processor_status: ProcessorStatus::initialize(),
            pc: 0,
            sp: 0,
            a: 0,
            x: 0,
            y: 0,
            cycle_count: 0,
        }
    }

    fn increment_pc(&mut self) {
        self.pc = self.pc.wrapping_add(1);
    }

    // Gets a byte and increments the program counter.
    fn fetch_next_byte(&mut self) -> u8 {
        let data = self.memory.get_byte(self.pc);
        self.increment_pc();
        data
    }

    // Fetches two byte $LL and $HH and increments the program counter twice
    // returning the u16 as $HHLL
    fn fetch_next_two_bytes(&mut self) -> u16 {
        let two_bytes = self.memory.get_two_bytes(self.pc);
        self.increment_pc();
        self.increment_pc();
        two_bytes
    }

    fn implied(&mut self, data: u8) -> AddressingMode {
        // Note we do not increment the PC!
        AddressingMode::Implied(data)
    }

    fn immediate(&mut self) -> AddressingMode {
        AddressingMode::Immediate(self.fetch_next_byte())
    }

    fn zero_page(&mut self) -> AddressingMode {
        AddressingMode::ZeroPage(self.fetch_next_byte())
    }

    fn zero_page_x(&mut self) -> AddressingMode {
        AddressingMode::IndexedZeroPageX(self.fetch_next_byte())
    }

    fn absolute(&mut self) -> AddressingMode {
        AddressingMode::Absolute(self.fetch_next_two_bytes())
    }

    fn absolute_x(&mut self) -> AddressingMode {
        AddressingMode::IndexedX(self.fetch_next_two_bytes())
    }

    fn absolute_y(&mut self) -> AddressingMode {
        AddressingMode::IndexedY(self.fetch_next_two_bytes())
    }

    fn indirect_zero_page_x(&mut self) -> AddressingMode {
        AddressingMode::IndexedZeroPageX(self.fetch_next_byte())
    }

    fn indirect_zero_page_y(&mut self) -> AddressingMode {
        AddressingMode::IndexedZeroPageY(self.fetch_next_byte())
    }

    // An instruction fetch will get the next instruction and increment the PC appropriately according to the instruction length.
    // We match on the opcode below to recieve a "FetchInstructionResult", which provides the appropriate opcode, addressing mode, and
    // number of base cycles for the operation.
    pub fn fetch_instruction(&mut self) -> FetchInstructionResult {
        let opcode_byte = self.fetch_next_byte();
        use Opcode::*;
        let (opcode, addressing_mode, cycles) = match opcode_byte {
            0x69 => (ADC, self.immediate(), 2),
            0x65 => (ADC, self.zero_page(), 3),
            0x75 => (ADC, self.zero_page_x(), 4),
            0x6D => (ADC, self.absolute(), 4),
            0x7D => (ADC, self.absolute_x(), 4),
            0x79 => (ADC, self.absolute_y(), 4),
            0x61 => (ADC, self.indirect_zero_page_x(), 6),
            0x71 => (ADC, self.indirect_zero_page_y(), 5),

            0x29 => (AND, self.immediate(), 2),
            0x25 => (AND, self.zero_page(), 3),
            0x35 => (AND, self.zero_page_x(), 4),
            0x2D => (AND, self.absolute(), 4),
            0x3D => (AND, self.absolute_x(), 4),
            0x39 => (AND, self.absolute_y(), 4),
            0x21 => (AND, self.indirect_zero_page_x(), 6),
            0x31 => (AND, self.indirect_zero_page_y(), 5),

            0x0A => (ASL, self.implied(self.a), 2),
            0x06 => (ASL, self.zero_page(), 5),
            0x16 => (ASL, self.zero_page_x(), 6),
            0x0E => (ASL, self.absolute(), 6),
            0x1E => (ASL, self.absolute_x(), 7),
            x => todo!("Unimplemented opcode: {:?}!", x),
        };
        FetchInstructionResult::new(opcode, addressing_mode, cycles)
    }

    // Checks and sets/clears the negative flag based on a byte.
    fn check_and_set_negative(&mut self, x: u8) {
        if (x as i8) < 0 {
            self.processor_status = self.processor_status.set_negative();
        } else {
            self.processor_status = self.processor_status.clear_negative();
        }
    }

    fn check_and_set_overflow(&mut self, sum: u16) {
        let s_sum = sum as i16;
        if s_sum < -128 || sum > 127 {
            self.processor_status = self.processor_status.set_overflow();
        } else {
            self.processor_status = self.processor_status.clear_overflow();
        }
    }

    // Checks and sets/clears the zero flag based on a byte.
    fn check_and_set_zero(&mut self, x: u8) {
        if x == 0 {
            self.processor_status = self.processor_status.set_zero();
        } else {
            self.processor_status = self.processor_status.clear_zero();
        }
    }

    fn check_and_set_carry(&mut self, c: bool) {
        if c {
            self.processor_status = self.processor_status.set_carry();
        } else {
            self.processor_status = self.processor_status.clear_carry();
        }
    }

    // Add With Carry
    // A <- A + M + C
    // Affects Flags: N, V, Z, C
    fn execute_adc(&mut self, m: u8) {
        let extended_result = self.a as u16 + m as u16 + self.processor_status.carry() as u16;
        let c = extended_result >> 8 & 0x0001 == 0x0001;

        let result = extended_result as u8;

        // Flags
        self.check_and_set_negative(result);
        self.check_and_set_overflow(extended_result);
        self.check_and_set_zero(result);
        self.check_and_set_carry(c);

        self.a = result as u8;
    }

    // Bitwise AND with Accumulator
    // A <- A & M
    // Affects Flags: N, Z
    fn execute_and(&mut self, m: u8) {
        let result = self.a & m;

        // Flags
        self.check_and_set_negative(result);
        self.check_and_set_zero(result);

        self.a = result;
    }

    // Arithmetic Shift Left
    // A <- M << 1
    // Affects Flags: N Z C
    fn execute_asl(&mut self, m: u8, write_location: WriteLocation) {
        let result = m << 1;

        // Flags
        self.check_and_set_negative(result);
        self.check_and_set_zero(result);
        // Check that the bit we'd shift left is 1.
        self.check_and_set_carry(m & 0x80 == 0x080);

        match write_location {
            WriteLocation::Accumulator => {
                self.a = result
            },
            WriteLocation::Memory(address) => {
                self.memory.set_byte(address, result);
            }
        }
    }

    pub fn fetch_instruction_and_execute(&mut self) {
        let FetchInstructionResult { opcode, addressing_mode, cycles } = self.fetch_instruction();
        // Now our PC is at the next instruction, so offsets will be measured relative to that.
        let AddressingModeData { data, address, page_boundary_result } = addressing_mode.into_data(self);
        let num_cycles = cycles + if page_boundary_result == PageBoundaryCrossed { 1 } else { 0 };

        // Now we can actually *execute* the instruction.
        match opcode {
            Opcode::ADC => self.execute_adc(data),
            Opcode::AND => self.execute_and(data),
            Opcode::ASL => {
                println!("0x{:04X}: 0x{:02X}", address.unwrap_or_default(), data);
                // We write to memory if we returned a specific address.
                let wl = if let Some(address) = address {
                    WriteLocation::Memory(address)
                } else {
                    WriteLocation::Accumulator
                };
                self.execute_asl(data, wl);
            },
            x => todo!("Unimplemented Opcode {:?}", x),
        }

        self.cycle_count += num_cycles;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adc() {
        let mut cpu = Cpu::initialize();
        cpu.a = 0x03F;
        cpu.processor_status = ProcessorStatus::initialize().set_carry();
        cpu.memory.set_bytes(0x00, &[0x69, 0x02]);
        cpu.fetch_instruction_and_execute();

        assert_eq!(0x42, cpu.a);
        assert_eq!(0x02, cpu.pc);
        assert!(!cpu.processor_status.is_overflow());
        assert!(!cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
        assert!(!cpu.processor_status.is_carry());
    }

    #[test]
    fn test_and() {
        let mut cpu = Cpu::initialize();
        cpu.a = 0xFF;
        cpu.memory.set_bytes(0x00, &[0x29, 0x42]);
        cpu.fetch_instruction_and_execute();

        assert_eq!(0x42, cpu.a);
        assert_eq!(0x02, cpu.pc);
        assert!(!cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
    }

    #[test]
    fn test_asl() {
        let mut cpu = Cpu::initialize();
        cpu.a = 0x80;
        cpu.memory.set_bytes(0x00, &[0x0A, 0x06, 0x42]);
        cpu.memory.set_byte(0x0042, 0x40);

        // One instruction should just left shift A and set the carry.
        cpu.fetch_instruction_and_execute();

        assert_eq!(0x01, cpu.pc);
        assert_eq!(0x00, cpu.a);
        assert!(!cpu.processor_status.is_negative());
        assert!(cpu.processor_status.is_zero());
        assert!(cpu.processor_status.is_carry());

        // Now we'll left shift a value directly on the zero page at 0x42.
        cpu.fetch_instruction_and_execute();

        assert_eq!(0x03, cpu.pc);
        assert_eq!(0x00, cpu.a);
        assert_eq!(0x80, cpu.memory.get_byte(0x0042));
        assert!(cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
        assert!(!cpu.processor_status.is_carry());
    }
}
