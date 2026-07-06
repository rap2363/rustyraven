use core::num;
use std::net::Shutdown::Write;

use crate::addressing_modes::{AddressingMode, AddressingModeData, PageBoundaryResult::PageBoundaryCrossed};
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
    BVC,
    BVS,
    CLC,
    CLD,
    CLI,
    CLV,
    CMP,
    CPX,
    CPY,
    DEC,
    DEX,
    DEY,
    EOR,
    INC,
    INX,
    INY,
    JMP,
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

enum WriteLocation {
    Accumulator,
    Memory(u16),
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
        self.memory.write_byte_to_stack(self.sp, data);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop_stack(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.memory.read_byte_from_stack(self.sp)
    }

    // Gets a byte and increments the program counter.
    fn fetch_next_byte(&mut self) -> u8 {
        let data = self.memory.read_byte(self.pc);
        self.increment_pc();
        data
    }

    // Fetches two byte $LL and $HH and increments the program counter twice
    // returning the u16 as $HHLL
    fn fetch_next_two_bytes(&mut self) -> u16 {
        let two_bytes = self.memory.read_two_bytes(self.pc);
        self.increment_pc();
        self.increment_pc();
        two_bytes
    }

    // Fetches two byte $LL and $HH and increments the program counter twice
    // returning the u16 as $HHLL
    // Note: This one will *wrap* around the existing page.
    fn fetch_next_two_bytes_wrapping_page(&mut self) -> u16 {
        let two_bytes = self.memory.read_two_bytes_wrapping_page(self.pc);
        self.increment_pc();
        self.increment_pc();
        two_bytes
    }

    fn implied(&mut self) -> AddressingMode {
        // Note we do not increment the PC because we don't need to fetch a new byte!
        AddressingMode::Implied
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

    fn zero_page_y(&mut self) -> AddressingMode {
        AddressingMode::IndexedZeroPageY(self.fetch_next_byte())
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
        AddressingMode::IndirectZeroPageX(self.fetch_next_byte())
    }

    fn indirect_zero_page_y(&mut self) -> AddressingMode {
        AddressingMode::IndirectZeroPageY(self.fetch_next_byte())
    }

    // Used *exclusively* for the JMP Indirect mode (0x6C).
    fn indirect(&mut self) -> AddressingMode {
        AddressingMode::Indirect(self.fetch_next_two_bytes())
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

            0x0A => (ASL, self.implied(), 2),
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

            0x00 => (BRK, self.implied(), 7),

            0x50 => (BVC, self.relative(), 2),

            0x70 => (BVS, self.relative(), 2),

            0x18 => (CLC, self.implied(), 2),

            0xD8 => (CLD, self.implied(), 2),
            
            0x58 => (CLI, self.implied(), 2),
            
            0xB8 => (CLV, self.implied(), 2),

            0xC9 => (CMP, self.immediate(), 2),
            0xC5 => (CMP, self.zero_page(), 3),
            0xD5 => (CMP, self.zero_page_x(), 4),
            0xCD => (CMP, self.absolute(), 4),
            0xDD => (CMP, self.absolute_x(), 4),
            0xD9 => (CMP, self.absolute_y(), 4),
            0xC1 => (CMP, self.indirect_zero_page_x(), 6),
            0xD1 => (CMP, self.indirect_zero_page_y(), 5),

            0xE0 => (CPX, self.immediate(), 2),
            0xE4 => (CPX, self.zero_page(), 3),
            0xEC => (CPX, self.absolute(), 4),

            0xC0 => (CPY, self.immediate(), 2),
            0xC4 => (CPY, self.zero_page(), 3),
            0xCC => (CPY, self.absolute(), 4),

            0xC6 => (DEC, self.zero_page(), 5),
            0xD6 => (DEC, self.zero_page_x(), 6),
            0xCE => (DEC, self.absolute(), 6),
            0xDE => (DEC, self.absolute_x(), 7),

            0xCA => (DEX, self.implied(), 2),

            0x88 => (DEY, self.implied(), 2),

            0x49 => (EOR, self.immediate(), 2),
            0x45 => (EOR, self.zero_page(), 3),
            0x55 => (EOR, self.zero_page_x(), 4),
            0x4D => (EOR, self.absolute(), 4),
            0x5D => (EOR, self.absolute_x(), 4),
            0x59 => (EOR, self.absolute_y(), 4),
            0x41 => (EOR, self.indirect_zero_page_x(), 6),
            0x51 => (EOR, self.indirect_zero_page_y(), 5),


            0xE6 => (INC, self.zero_page(), 5),
            0xF6 => (INC, self.zero_page_x(), 6),
            0xEE => (INC, self.absolute(), 6),
            0xFE => (INC, self.absolute_x(), 7),

            0xE8 => (INX, self.implied(), 2),

            0xC8 => (INY, self.implied(), 2),

            0x4C => (JMP, self.absolute(), 3),
            0x6C => (JMP, self.indirect(), 5),

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
                self.memory.write_byte(address, result);
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
        self.processor_status = self.processor_status.set_break();
        let pc_plus_two = self.pc.wrapping_add(2);
        // Little Endian, push the low bits, then the high ones.
        self.push_stack(pc_plus_two as u8);
        self.push_stack((pc_plus_two >> 4) as u8);
        self.push_stack(self.processor_status.into());
    }

    // Branch on Overflow Clear
    // branch on V = 0
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bvc(&mut self, m: u8) -> bool {
        if !self.processor_status.is_overflow() {
            self.pc = self.pc.wrapping_add(m as u16);
            true
        } else {
            false
        }
    }

    // Branch on Overflow Set
    // branch on V = 1
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bvs(&mut self, m: u8) -> bool {
        if self.processor_status.is_overflow() {
            self.pc = self.pc.wrapping_add(m as u16);
            true
        } else {
            false
        }
    }

    // Clear carry flag
    // C = 0
    // Affects Flags: C
    fn clc(&mut self) {
        self.processor_status = self.processor_status.clear_carry();
    }

    // Clear decimal mode flag
    // D = 0
    // Affects Flags: D
    fn cld(&mut self) {
        self.processor_status = self.processor_status.clear_decimal();
    }

    // Clear interrupt flag
    // I = 0
    // Affects Flags: I
    fn cli(&mut self) {
        self.processor_status = self.processor_status.clear_interrupt();
    }

    // Clear overflow flag
    // V = 0
    // Affects Flags: V
    fn clv(&mut self) {
        self.processor_status = self.processor_status.clear_overflow();
    }

    // Helper method for the cmp ops.
    fn cmp_processor_status(&self, data: i8) -> ProcessorStatus {
        if data > 0 {
            self.processor_status.set_carry().clear_negative().clear_zero()
        } else if data < 0 {
            self.processor_status.set_negative().clear_carry().clear_zero()
        } else {
            self.processor_status.set_zero().clear_carry().clear_negative()
        }
    }

    // Compare accumulator to memory (this *only* sets flags based on the value of A - M).
    // This will set the carry if A - M > 0, the negative flag if A - M < 0, and the zero flag if A - M = 0.
    // A - M
    // Affects Flags: N Z C
    fn cmp(&mut self, data: u8) {
        self.processor_status = self.cmp_processor_status((self.a as i8) - (data as i8));
    }

    // Compare X register to memory (this *only* sets flags based on the value of X - M).
    // This will set the carry if X - M > 0, the negative flag if X - M < 0, and the zero flag if X - M = 0.
    // X - M
    // Affects Flags: N Z C
    fn cpx(&mut self, data: u8) {
        self.processor_status = self.cmp_processor_status((self.x as i8) - (data as i8));
    }

    // Compare accumulator to memory (this *only* sets flags based on the value of Y - M).
    // This will set the carry if Y - M > 0, the negative flag if Y - M < 0, and the zero flag if Y - M = 0.
    // Y - M
    // Affects Flags: N Z C
    fn cpy(&mut self, data: u8) {
        self.processor_status = self.cmp_processor_status((self.y as i8) - (data as i8));
    }

    // Decrement memory by one. Requires us to *write* to a location in memory.
    // M <- M - 1
    // Affects Flags: N Z
    fn dec(&mut self, data: u8, address: u16) {
        let result = data.wrapping_sub(1);
        self.check_and_set_negative(result);
        self.check_and_set_zero(result);

        self.memory.write_byte(address, result);
    }

    // Decrement the X register by one.
    // X <- X - 1
    // Affects Flags: N Z
    fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.check_and_set_negative(self.x);
        self.check_and_set_zero(self.x);
    }

    // Decrement the Y register by one.
    // Y <- Y - 1
    // Affects Flags: N Z
    fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.check_and_set_negative(self.y);
        self.check_and_set_zero(self.y);
    }

    // Exclusive OR A with M.
    // A <- A xor M
    // Affects flags: N Z
    fn eor(&mut self, data: u8) {
        self.a = self.a ^ data;
        self.check_and_set_negative(self.a);
        self.check_and_set_zero(self.a);
    }
    
    // Increment memory by one. Requires us to *write* to a location in memory.
    // M <- M + 1
    // Affects Flags: N Z
    fn inc(&mut self, data: u8, address: u16) {
        let result = data.wrapping_add(1);
        self.check_and_set_negative(result);
        self.check_and_set_zero(result);

        self.memory.write_byte(address, result);
    }

    // Increment the X register by one.
    // X <- X + 1
    // Affects Flags: N Z
    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.check_and_set_negative(self.x);
        self.check_and_set_zero(self.x);
    }

    // Increment the Y register by one.
    // Y <- Y + 1
    // Affects Flags: N Z
    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.check_and_set_negative(self.y);
        self.check_and_set_zero(self.y);
    }

    // Jump to a new location
    // PC <- $HHLL
    // Affects Flags: (none)
    fn jmp(&mut self, address: u16) {
        self.pc = address;
    }

    // ----------- Instruction Fetching ----------- //

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
                let (data, wl) = if let Some(address) = address {
                    (data, WriteLocation::Memory(address))
                } else {
                    (self.a, WriteLocation::Accumulator)
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
            Opcode::BVC => Self::calculate_branch_cycles(&mut num_cycles, self.bvc(data), pbc),
            Opcode::BVS => Self::calculate_branch_cycles(&mut num_cycles, self.bvs(data), pbc),
            Opcode::CLC => self.clc(),
            Opcode::CLD => self.cld(),
            Opcode::CLI => self.cli(),
            Opcode::CLV => self.clv(),
            Opcode::CMP => self.cmp(data),
            Opcode::CPX => self.cpx(data),
            Opcode::CPY => self.cpy(data),
            Opcode::DEC => self.dec(data, address.expect("Address should be supplied for a DEC!")),
            Opcode::DEX => self.dex(),
            Opcode::DEY => self.dey(),
            Opcode::EOR => self.eor(data),
            Opcode::INC => self.dec(data, address.expect("Address should be supplied for a INC!")),
            Opcode::INX => self.dex(),
            Opcode::INY => self.dey(),
            Opcode::JMP => self.jmp(address.expect("Address should have been supplied for a JMP!")),
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
        cpu.memory.write_bytes(0x00, &[0x69, 0x02]);
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
        cpu.memory.write_bytes(0x00, &[0x29, 0x42]);
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
        cpu.memory.write_bytes(0x00, &[0x0A, 0x06, 0x42]);
        cpu.memory.write_byte(0x0042, 0x40);

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
        assert_eq!(0x80, cpu.memory.read_byte(0x0042));
        assert!(cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
        assert!(!cpu.processor_status.is_carry());
    }

    #[test]
    fn test_bcc_and_bcs() {
        let mut cpu = Cpu::initialize();
        cpu.memory.write_bytes(0x00, &[0x90, 0x40]);
        cpu.memory.write_bytes(0x0042, &[0xB0, 0xFF, 0xB0, 0xFF]);

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
        cpu.memory.write_bytes(0x00, &[0x24, 0x42]);
        cpu.memory.write_byte(0x0042, 0xF0);

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
        assert!(cpu.processor_status.is_break());
        assert_eq!(7, cpu.cycle_count);
        assert_eq!(0xFC, cpu.sp);
        assert_eq!(0x03, cpu.memory.read_byte(0x10FF));
        assert_eq!(0x00, cpu.memory.read_byte(0x10FE));
        assert_eq!(0x52, cpu.memory.read_byte(0x10FD));
    }

    #[test]
    fn test_cmp() {
        let mut cpu = Cpu::initialize();
        cpu.a = 0x42;
        cpu.memory.write_bytes(0x00, &[0xC9, 0x43, 0xC9, 0x42, 0xC9, 0x41]);

        cpu.fetch_instruction_and_execute();
        assert!(cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
        assert!(!cpu.processor_status.is_carry());

        cpu.fetch_instruction_and_execute();
        assert!(!cpu.processor_status.is_negative());
        assert!(cpu.processor_status.is_zero());
        assert!(!cpu.processor_status.is_carry());

        cpu.fetch_instruction_and_execute();
        assert!(!cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
        assert!(cpu.processor_status.is_carry());
    }

    #[test]
    fn test_dec() {
        let mut cpu = Cpu::initialize();
        // We'll decrement three times (the last one will be using the absolute addressing mode)
        cpu.memory.write_bytes(0x00, &[0xC6, 0x42, 0xC6, 0x42, 0xCE, 0x42, 0x00]);
        cpu.memory.write_byte(0x42, 0x02);

        cpu.fetch_instruction_and_execute();
        assert!(!cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
        assert_eq!(cpu.memory.read_byte(0x0042), 0x01);
        assert_eq!(5, cpu.cycle_count);

        cpu.fetch_instruction_and_execute();
        assert!(!cpu.processor_status.is_negative());
        assert!(cpu.processor_status.is_zero());
        assert_eq!(cpu.memory.read_byte(0x0042), 0x00);
        assert_eq!(10, cpu.cycle_count);

        cpu.fetch_instruction_and_execute();
        assert!(cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
        assert_eq!(cpu.memory.read_byte(0x0042), 0xFF);
        assert_eq!(16, cpu.cycle_count);
    }

    #[test]
    fn test_jmp() {
        let mut cpu = Cpu::initialize();
        cpu.memory.write_bytes(0x00, &[0x4C, 0x34, 0x12]);

        cpu.fetch_instruction_and_execute();
        assert_eq!(cpu.pc, 0x1234);
        assert_eq!(3, cpu.cycle_count);
    }

    #[test]
    fn test_jmp_indirect() {
        let mut cpu = Cpu::initialize();
        cpu.memory.write_bytes(0x00, &[0x6C, 0xFF, 0x11]);
        cpu.memory.write_byte(0x11FF, 0x34);
        cpu.memory.write_byte(0x1100, 0x12);
        cpu.memory.write_byte(0x1234, 0xEF);
        cpu.memory.write_byte(0x1235, 0xBE);
        cpu.fetch_instruction_and_execute();

        assert_eq!(cpu.pc, 0xBEEF);
        assert_eq!(5, cpu.cycle_count);
    }
}
