use core::num;
use std::net::Shutdown::Write;

use crate::addressing_modes::{AddressingMode, AddressingModeData, PageBoundaryResult::PageBoundaryCrossed, WriteLocation};
use crate::memory::CpuMemory;
use crate::processor_status::ProcessorStatus;

#[derive(Debug)]
enum Opcode {
    ADC,
    AND,
    ASL,
    BCC,
    BCS,
    BEQ,
    BIT,
    BMI,
    BNE,
    BPL,
    BRK,
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
            sp: 0xFF,
            a: 0,
            x: 0,
            y: 0,
            cycle_count: 0,
        }
    }

    fn increment_pc(&mut self) {
        self.pc = self.pc.wrapping_add(1);
    }

    fn push_stack(&mut self, data: u8) {
        self.memory.set_byte(0x1000 + (self.sp as u16), data);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop_stack(&mut self) -> u8 {
        let data = self.memory.get_byte(0x1000 + (self.sp as u16));
        self.sp = self.sp.wrapping_add(1);
        data
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

    fn relative(&mut self) -> AddressingMode {
        AddressingMode::Relative(self.fetch_next_byte())
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

            0x90 => (BCC, self.relative(), 2),

            0xB0 => (BCS, self.relative(), 2),
            
            0xF0 => (BEQ, self.relative(), 2),

            0x24 => (BIT, self.zero_page(), 3),
            0x2C => (BIT, self.absolute(), 4),

            0x30 => (BMI, self.relative(), 2),

            0xD0 => (BNE, self.relative(), 2),
            
            0x10 => (BPL, self.relative(), 2),

            0x00 => (BRK, self.implied(0x00), 7),

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
    fn adc(&mut self, m: u8) {
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
    fn and(&mut self, m: u8) {
        let result = self.a & m;

        // Flags
        self.check_and_set_negative(result);
        self.check_and_set_zero(result);

        self.a = result;
    }

    // Arithmetic Shift Left
    // A <- M << 1
    // Affects Flags: N Z C
    fn asl(&mut self, m: u8, write_location: WriteLocation) {
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

    // Branch on Carry Clear
    // branch on C = 0
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bcc(&mut self, m: u8) -> bool {
        if !self.processor_status.is_carry() {
            self.pc = self.pc.wrapping_add(m as u16);
            true
        } else {
            false
        }
    }

    // Branch on Carry Set
    // branch on C = 1
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bcs(&mut self, m: u8) -> bool {
        if self.processor_status.is_carry() {
            self.pc = self.pc.wrapping_add(m as u16);
            true
        } else {
            false
        }
    }

    // Branch on Zero Flag Set
    // branch on Z = 1
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn beq(&mut self, m: u8) -> bool {
        if self.processor_status.is_zero() {
            self.pc = self.pc.wrapping_add(m as u16);
            true
        } else {
            false
        }
    }

    // Bit Test
    // Sets the zero flag based on the value of A & M.
    // Also sets N and V based on bits 7 and 6 of M respectively.
    // Affects Flags: N V Z
    // Returns a bool for whether or not we branch.
    fn bit(&mut self, m: u8) {
        self.check_and_set_negative(m);
        println!("0x{:02X}", m);
        if m >> 6 & 0x01 == 0x01 {
            self.processor_status = self.processor_status.set_overflow();
        } else {
            self.processor_status = self.processor_status.clear_overflow();
        }
        self.check_and_set_zero(self.a & m);
    }

    // Branch on Result Minus
    // branch on N = 1
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bmi(&mut self, m: u8) -> bool {
        if self.processor_status.is_negative() {
            self.pc = self.pc.wrapping_add(m as u16);
            true
        } else {
            false
        }
    }

    // Branch on Result Not Zero
    // branch on Z = 0
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bne(&mut self, m: u8) -> bool {
        if !self.processor_status.is_zero() {
            self.pc = self.pc.wrapping_add(m as u16);
            true
        } else {
            false
        }
    }

    // Branch on Result Plus
    // branch on N = 0
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bpl(&mut self, m: u8) -> bool {
        if !self.processor_status.is_negative() {
            self.pc = self.pc.wrapping_add(m as u16);
            true
        } else {
            false
        }
    }

    // Forces a software interrupt. We do the following:
    // 1. Set the interrupt flag,
    // 2. Push the PC+2 to the stack (return address is 2 bytes after the current PC!)
    // 3. Push the status register to the stack
    fn brk(&mut self) {
        self.processor_status = self.processor_status.set_interrupt();
        let pc_plus_two = self.pc.wrapping_add(2);
        // Little Endian, push the low bits, then the high ones.
        self.push_stack(pc_plus_two as u8);
        self.push_stack((pc_plus_two >> 4) as u8);
        self.push_stack(self.processor_status.into());
    }

    // A bit of a hack to deal with the variability of branch cycles.
    fn calculate_branch_cycles(num_cycles: &mut usize, branch: bool, pbc: bool) {
        // Cycle Calculation
        // Branch | PBR | Cycles
        //    F   |  F  | 2
        //    F   |  T  | 2
        //    T   |  F  | 3
        //    T   |  T  | 4
        *num_cycles = if branch {
            if pbc { 4 } else { 3 }
        } else {
            2
        }
    }

    pub fn fetch_instruction_and_execute(&mut self) {
        let FetchInstructionResult { opcode, addressing_mode, cycles } = self.fetch_instruction();
        // Now our PC is at the next instruction, so offsets will be measured relative to that.
        let AddressingModeData { data, address, page_boundary_result } = addressing_mode.into_data(self);
        let pbc = page_boundary_result == PageBoundaryCrossed;
        let mut num_cycles: usize = cycles + if pbc { 1 } else { 0 };
        // Now we can actually *execute* the instruction.
        match opcode {
            Opcode::ADC => self.adc(data),
            Opcode::AND => self.and(data),
            Opcode::ASL => {
                // We write to memory if we returned a specific address.
                let wl = if let Some(address) = address {
                    WriteLocation::Memory(address)
                } else {
                    WriteLocation::Accumulator
                };
                self.asl(data, wl);
            },
            Opcode::BCC => Self::calculate_branch_cycles(&mut num_cycles, self.bcc(data), pbc),
            Opcode::BCS => Self::calculate_branch_cycles(&mut num_cycles, self.bcs(data), pbc),
            Opcode::BEQ => Self::calculate_branch_cycles(&mut num_cycles, self.beq(data), pbc),
            Opcode::BIT => self.bit(data),
            Opcode::BMI => Self::calculate_branch_cycles(&mut num_cycles, self.bmi(data), pbc),
            Opcode::BNE => Self::calculate_branch_cycles(&mut num_cycles, self.bne(data), pbc),
            Opcode::BPL => Self::calculate_branch_cycles(&mut num_cycles, self.bpl(data), pbc),
            Opcode::BRK => self.brk(),

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

    #[test]
    fn test_bcc_and_bcs() {
        let mut cpu = Cpu::initialize();
        cpu.memory.set_bytes(0x00, &[0x90, 0x40]);
        cpu.memory.set_bytes(0x0042, &[0xB0, 0xFF, 0xB0, 0xFF]);

        // One instruction should just branch the CPU to address 0x0042.
        // Cycles should be 3 because we branched.
        cpu.fetch_instruction_and_execute();

        assert_eq!(0x0042, cpu.pc);
        assert_eq!(3, cpu.cycle_count);
        cpu.fetch_instruction_and_execute();
        cpu.processor_status = cpu.processor_status.set_carry();
        cpu.fetch_instruction_and_execute();

        assert_eq!(0x0046 + 0x00FF, cpu.pc);
        assert_eq!(3 + 2 + 4, cpu.cycle_count);
        // After fetching two more we will have *not* branched once and then branched
        // after the carry was set.
    }


    #[test]
    fn test_bit() {
        let mut cpu = Cpu::initialize();
        cpu.a = 0x0F;
        cpu.memory.set_bytes(0x00, &[0x24, 0x42]);
        cpu.memory.set_byte(0x0042, 0xF0);

        cpu.fetch_instruction_and_execute();
        assert!(cpu.processor_status.is_overflow());
        assert!(cpu.processor_status.is_negative());
        assert!(cpu.processor_status.is_zero());
        assert_eq!(3, cpu.cycle_count);
    }

    #[test]
    fn test_break() {
        let mut cpu = Cpu::initialize();
        cpu.processor_status = ProcessorStatus::from(0x42);
        cpu.fetch_instruction_and_execute();

        // Now we should have pushed our PC and the processor status to the stack.
        assert_eq!(0x01, cpu.pc);
        assert!(cpu.processor_status.is_interrupt());
        assert_eq!(7, cpu.cycle_count);
        assert_eq!(0xFC, cpu.sp);
        assert_eq!(0x03, cpu.memory.get_byte(0x10FF));
        assert_eq!(0x00, cpu.memory.get_byte(0x10FE));
        assert_eq!(ProcessorStatus::from(0x42).set_interrupt().into(), cpu.memory.get_byte(0x10FD));
    }
}
