use crate::addressing_modes::PageBoundaryResult;
use crate::addressing_modes::{AddressingMode, AddressingModeData, PageBoundaryResult::PageBoundaryCrossed};
use crate::memory::CpuMemory;
use crate::processor_status::ProcessorStatus;

const NMI_ADDRESS: u16 = 0xFFFA;
const RESET_ADDRESS: u16 = 0xFFFC;

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
    JSR,
    LDA,
    LDX,
    LDY,
    LSR,
    NOP,
    ORA,
    PHA,
    PHP,
    PLA,
    PLP,
    ROL,
    ROR,
    RTI,
    RTS,
    SBC,
    SEC,
    SED,
    SEI,
    STA,
    STX,
    STY,
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
}

#[derive(Debug, PartialEq)]
enum Nmi {
    None,
    CycleLatency(u8),
    Interrupt,
}

pub struct Cpu {
    pub memory: CpuMemory,
    pub processor_status: ProcessorStatus,
    pub pc: u16,
    pub sp: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub cycle_count: usize,
    pub cycle_budget: i8, // when cycle_budget >= 0, we're allowed to execute cycles.
    nmi: Nmi, // Starts at 7 and decrements down to 0 with our normal cycle count. When this
              // happens we trigger an interrupt!
}

enum Cycles {
    Fixed(usize),
    PageCrossing(usize),
}

pub struct FetchInstructionResult {
    opcode: Opcode,
    addressing_mode: AddressingMode,
    cycles: Cycles,
}

enum WriteLocation {
    Accumulator,
    Memory(u16),
}

impl FetchInstructionResult {
    fn new(opcode: Opcode, addressing_mode: AddressingMode, cycles: Cycles) -> Self {
        Self { opcode, addressing_mode, cycles }
    }
}

impl Cpu {
    pub fn initialize() -> Self {
        Self {
            memory: CpuMemory::initialize(),
            processor_status: ProcessorStatus::initialize(),
            pc: RESET_ADDRESS,
            sp: 0xFD,
            a: 0,
            x: 0,
            y: 0,
            cycle_count: 0,
            cycle_budget: 0,
            nmi: Nmi::None,
        }
    }

    pub fn to_string(&self) -> String {
        format!("PC:0x{:04X}, A:0x{:02X}, X:0x{:02X}, Y:0x{:02X}, P:0x{:02X}, SP:0x{:02X}, CYC:{}, cyc_budget:{}, nmi:{:?}", self.pc, self.a, self.x, self.y, self.processor_status.into(), self.sp, self.cycle_count, self.cycle_budget, self.nmi)
    }

    pub fn set_nmi(&mut self) {
        self.nmi = Nmi::CycleLatency(7); // Interrupt latency with 7 cycles.
    }

    fn dec_nmi(&mut self, cycles: u8) {
        if let Nmi::CycleLatency(n) = self.nmi {
            self.nmi = if cycles >= n {
                Nmi::Interrupt
            } else {
                Nmi::CycleLatency(n - cycles)
            }
        }
    }

    fn is_nmi(&mut self) -> bool {
        self.nmi == Nmi::Interrupt
    }

    fn clear_nmi(&mut self) {
        self.nmi = Nmi::None;
    }

    fn increment_pc(&mut self) {
        self.pc = self.pc.wrapping_add(1);
    }

    fn push_stack(&mut self, m: u8) {
        self.memory.write_byte_to_stack(self.sp, m);
        self.sp = self.sp.wrapping_sub(1);
    }

    // Pushes the current PC to the stack. First $HH then $LL
    fn push_pc(&mut self) {
        let lo = self.pc as u8;
        let hi = (self.pc >> 8) as u8;
        self.push_stack(hi);
        self.push_stack(lo);
    }

    fn pull_stack(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.memory.read_byte_from_stack(self.sp)
    }

    // Pulls an address from the stack: first $LL then $HH
    fn pull_address(&mut self) -> u16 {
        u16::from_le_bytes([self.pull_stack(), self.pull_stack()])
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
            0x69 => (ADC, self.immediate(), Cycles::Fixed(2)),
            0x65 => (ADC, self.zero_page(), Cycles::Fixed(3)),
            0x75 => (ADC, self.zero_page_x(), Cycles::Fixed(4)),
            0x6D => (ADC, self.absolute(), Cycles::Fixed(4)),
            0x7D => (ADC, self.absolute_x(), Cycles::PageCrossing(4)),
            0x79 => (ADC, self.absolute_y(), Cycles::PageCrossing(4)),
            0x61 => (ADC, self.indirect_zero_page_x(), Cycles::Fixed(6)),
            0x71 => (ADC, self.indirect_zero_page_y(), Cycles::PageCrossing(5)),

            0x29 => (AND, self.immediate(), Cycles::Fixed(2)),
            0x25 => (AND, self.zero_page(), Cycles::Fixed(3)),
            0x35 => (AND, self.zero_page_x(), Cycles::Fixed(4)),
            0x2D => (AND, self.absolute(), Cycles::Fixed(4)),
            0x3D => (AND, self.absolute_x(), Cycles::PageCrossing(4)),
            0x39 => (AND, self.absolute_y(), Cycles::PageCrossing(4)),
            0x21 => (AND, self.indirect_zero_page_x(), Cycles::Fixed(6)),
            0x31 => (AND, self.indirect_zero_page_y(), Cycles::PageCrossing(5)),

            0x0A => (ASL, self.implied(), Cycles::Fixed(2)),
            0x06 => (ASL, self.zero_page(), Cycles::Fixed(5)),
            0x16 => (ASL, self.zero_page_x(), Cycles::Fixed(6)),
            0x0E => (ASL, self.absolute(), Cycles::Fixed(6)),
            0x1E => (ASL, self.absolute_x(), Cycles::Fixed(7)),

            0x90 => (BCC, self.relative(), Cycles::Fixed(2)),

            0xB0 => (BCS, self.relative(), Cycles::Fixed(2)),
            
            0xF0 => (BEQ, self.relative(), Cycles::Fixed(2)),

            0x24 => (BIT, self.zero_page(), Cycles::Fixed(3)),
            0x2C => (BIT, self.absolute(), Cycles::Fixed(4)),

            0x30 => (BMI, self.relative(), Cycles::Fixed(2)),

            0xD0 => (BNE, self.relative(), Cycles::Fixed(2)),
            
            0x10 => (BPL, self.relative(), Cycles::Fixed(2)),

            0x00 => (BRK, self.implied(), Cycles::Fixed(7)),

            0x50 => (BVC, self.relative(), Cycles::Fixed(2)),

            0x70 => (BVS, self.relative(), Cycles::Fixed(2)),

            0x18 => (CLC, self.implied(), Cycles::Fixed(2)),

            0xD8 => (CLD, self.implied(), Cycles::Fixed(2)),
            
            0x58 => (CLI, self.implied(), Cycles::Fixed(2)),
            
            0xB8 => (CLV, self.implied(), Cycles::Fixed(2)),

            0xC9 => (CMP, self.immediate(), Cycles::Fixed(2)),
            0xC5 => (CMP, self.zero_page(), Cycles::Fixed(3)),
            0xD5 => (CMP, self.zero_page_x(), Cycles::Fixed(4)),
            0xCD => (CMP, self.absolute(), Cycles::Fixed(4)),
            0xDD => (CMP, self.absolute_x(), Cycles::PageCrossing(4)),
            0xD9 => (CMP, self.absolute_y(), Cycles::PageCrossing(4)),
            0xC1 => (CMP, self.indirect_zero_page_x(), Cycles::Fixed(6)),
            0xD1 => (CMP, self.indirect_zero_page_y(), Cycles::PageCrossing(5)),

            0xE0 => (CPX, self.immediate(), Cycles::Fixed(2)),
            0xE4 => (CPX, self.zero_page(), Cycles::Fixed(3)),
            0xEC => (CPX, self.absolute(), Cycles::Fixed(4)),

            0xC0 => (CPY, self.immediate(), Cycles::Fixed(2)),
            0xC4 => (CPY, self.zero_page(), Cycles::Fixed(3)),
            0xCC => (CPY, self.absolute(), Cycles::Fixed(4)),

            0xC6 => (DEC, self.zero_page(), Cycles::Fixed(5)),
            0xD6 => (DEC, self.zero_page_x(), Cycles::Fixed(6)),
            0xCE => (DEC, self.absolute(), Cycles::Fixed(6)),
            0xDE => (DEC, self.absolute_x(), Cycles::Fixed(7)),

            0xCA => (DEX, self.implied(), Cycles::Fixed(2)),

            0x88 => (DEY, self.implied(), Cycles::Fixed(2)),

            0x49 => (EOR, self.immediate(), Cycles::Fixed(2)),
            0x45 => (EOR, self.zero_page(), Cycles::Fixed(3)),
            0x55 => (EOR, self.zero_page_x(), Cycles::Fixed(4)),
            0x4D => (EOR, self.absolute(), Cycles::Fixed(4)),
            0x5D => (EOR, self.absolute_x(), Cycles::PageCrossing(4)),
            0x59 => (EOR, self.absolute_y(), Cycles::PageCrossing(4)),
            0x41 => (EOR, self.indirect_zero_page_x(), Cycles::Fixed(6)),
            0x51 => (EOR, self.indirect_zero_page_y(), Cycles::PageCrossing(5)),


            0xE6 => (INC, self.zero_page(), Cycles::Fixed(5)),
            0xF6 => (INC, self.zero_page_x(), Cycles::Fixed(6)),
            0xEE => (INC, self.absolute(), Cycles::Fixed(6)),
            0xFE => (INC, self.absolute_x(), Cycles::Fixed(7)),

            0xE8 => (INX, self.implied(), Cycles::Fixed(2)),

            0xC8 => (INY, self.implied(), Cycles::Fixed(2)),

            0x4C => (JMP, self.absolute(), Cycles::Fixed(3)),
            0x6C => (JMP, self.indirect(), Cycles::Fixed(5)),

            0x20 => (JSR, self.absolute(), Cycles::Fixed(6)),

            0xA9 => (LDA, self.immediate(), Cycles::Fixed(2)),
            0xA5 => (LDA, self.zero_page(), Cycles::Fixed(3)),
            0xB5 => (LDA, self.zero_page_x(), Cycles::Fixed(4)),
            0xAD => (LDA, self.absolute(), Cycles::Fixed(4)),
            0xBD => (LDA, self.absolute_x(), Cycles::PageCrossing(4)),
            0xB9 => (LDA, self.absolute_y(), Cycles::PageCrossing(4)),
            0xA1 => (LDA, self.indirect_zero_page_x(), Cycles::Fixed(6)),
            0xB1 => (LDA, self.indirect_zero_page_y(), Cycles::PageCrossing(5)),    

            0xA2 => (LDX, self.immediate(), Cycles::Fixed(2)),
            0xA6 => (LDX, self.zero_page(), Cycles::Fixed(3)),
            0xB6 => (LDX, self.zero_page_y(), Cycles::Fixed(4)),
            0xAE => (LDX, self.absolute(), Cycles::Fixed(4)),
            0xBE => (LDX, self.absolute_y(), Cycles::PageCrossing(4)),

            0xA0 => (LDY, self.immediate(), Cycles::Fixed(2)),
            0xA4 => (LDY, self.zero_page(), Cycles::Fixed(3)),
            0xB4 => (LDY, self.zero_page_x(), Cycles::Fixed(4)),
            0xAC => (LDY, self.absolute(), Cycles::Fixed(4)),
            0xBC => (LDY, self.absolute_x(), Cycles::PageCrossing(4)),

            0x4A => (LSR, self.implied(), Cycles::Fixed(2)),
            0x46 => (LSR, self.zero_page(), Cycles::Fixed(5)),
            0x56 => (LSR, self.zero_page_x(), Cycles::Fixed(6)),
            0x4E => (LSR, self.absolute(), Cycles::Fixed(6)),
            0x5E => (LSR, self.absolute_x(), Cycles::Fixed(7)),

            // A ton of NOOP's
            0xEA => (NOP, self.implied(), Cycles::Fixed(2)),
            0x1A => (NOP, self.implied(), Cycles::Fixed(2)),
            0x3A => (NOP, self.implied(), Cycles::Fixed(2)),
            0x5A => (NOP, self.implied(), Cycles::Fixed(2)),
            0x7A => (NOP, self.implied(), Cycles::Fixed(2)),
            0xDA => (NOP, self.implied(), Cycles::Fixed(2)),
            0xFA => (NOP, self.implied(), Cycles::Fixed(2)),
            0x80 => (NOP, self.immediate(), Cycles::Fixed(2)),
            0x82 => (NOP, self.immediate(), Cycles::Fixed(2)),
            0x89 => (NOP, self.immediate(), Cycles::Fixed(2)),
            0xC2 => (NOP, self.immediate(), Cycles::Fixed(2)),
            0xE2 => (NOP, self.immediate(), Cycles::Fixed(2)),
            0x04 => (NOP, self.zero_page(), Cycles::Fixed(3)),
            0x44 => (NOP, self.zero_page(), Cycles::Fixed(3)),
            0x64 => (NOP, self.zero_page(), Cycles::Fixed(3)),
            0x14 => (NOP, self.zero_page_x(), Cycles::Fixed(4)),
            0x34 => (NOP, self.zero_page_x(), Cycles::Fixed(4)),
            0x54 => (NOP, self.zero_page_x(), Cycles::Fixed(4)),
            0x74 => (NOP, self.zero_page_x(), Cycles::Fixed(4)),
            0xD4 => (NOP, self.zero_page_x(), Cycles::Fixed(4)),
            0xF4 => (NOP, self.zero_page_x(), Cycles::Fixed(4)),
            0x0C => (NOP, self.absolute(), Cycles::Fixed(4)),
            0x1C => (NOP, self.absolute_x(), Cycles::PageCrossing(4)),
            0x3C => (NOP, self.absolute_x(), Cycles::PageCrossing(4)),
            0x5C => (NOP, self.absolute_x(), Cycles::PageCrossing(4)),
            0x7C => (NOP, self.absolute_x(), Cycles::PageCrossing(4)),
            0xDC => (NOP, self.absolute_x(), Cycles::PageCrossing(4)),
            0xFC => (NOP, self.absolute_x(), Cycles::PageCrossing(4)),

            0x09 => (ORA, self.immediate(), Cycles::Fixed(2)),
            0x05 => (ORA, self.zero_page(), Cycles::Fixed(3)),
            0x15 => (ORA, self.zero_page_x(), Cycles::Fixed(4)),
            0x0D => (ORA, self.absolute(), Cycles::Fixed(4)),
            0x1D => (ORA, self.absolute_x(), Cycles::PageCrossing(4)),
            0x19 => (ORA, self.absolute_y(), Cycles::PageCrossing(4)),
            0x01 => (ORA, self.indirect_zero_page_x(), Cycles::Fixed(6)),
            0x11 => (ORA, self.indirect_zero_page_y(), Cycles::PageCrossing(5)),

            0x48 => (PHA, self.implied(), Cycles::Fixed(3)),

            0x08 => (PHP, self.implied(), Cycles::Fixed(3)),

            0x68 => (PLA, self.implied(), Cycles::Fixed(4)),

            0x28 => (PLP, self.implied(), Cycles::Fixed(4)),

            0x2A => (ROL, self.implied(), Cycles::Fixed(2)),
            0x26 => (ROL, self.zero_page(), Cycles::Fixed(5)),
            0x36 => (ROL, self.zero_page_x(), Cycles::Fixed(6)),
            0x2E => (ROL, self.absolute(), Cycles::Fixed(6)),
            0x3E => (ROL, self.absolute_x(), Cycles::Fixed(7)),

            0x6A => (ROR, self.implied(), Cycles::Fixed(2)),
            0x66 => (ROR, self.zero_page(), Cycles::Fixed(5)),
            0x76 => (ROR, self.zero_page_x(), Cycles::Fixed(6)),
            0x6E => (ROR, self.absolute(), Cycles::Fixed(6)),
            0x7E => (ROR, self.absolute_x(), Cycles::Fixed(7)),

            0x40 => (RTI, self.implied(), Cycles::Fixed(6)),

            0x60 => (RTS, self.implied(), Cycles::Fixed(6)),

            0xE9 => (SBC, self.immediate(), Cycles::Fixed(2)),
            0xE5 => (SBC, self.zero_page(), Cycles::Fixed(3)),
            0xF5 => (SBC, self.zero_page_x(), Cycles::Fixed(4)),
            0xED => (SBC, self.absolute(), Cycles::Fixed(4)),
            0xFD => (SBC, self.absolute_x(), Cycles::PageCrossing(4)),
            0xF9 => (SBC, self.absolute_y(), Cycles::PageCrossing(4)),
            0xE1 => (SBC, self.indirect_zero_page_x(), Cycles::Fixed(6)),
            0xF1 => (SBC, self.indirect_zero_page_y(), Cycles::PageCrossing(5)),

            0x38 => (SEC, self.implied(), Cycles::Fixed(2)),

            0xF8 => (SED, self.implied(), Cycles::Fixed(2)),

            0x78 => (SEI, self.implied(), Cycles::Fixed(2)),

            0x85 => (STA, self.zero_page(), Cycles::Fixed(3)),
            0x95 => (STA, self.zero_page_x(), Cycles::Fixed(4)),
            0x8D => (STA, self.absolute(), Cycles::Fixed(4)),
            0x9D => (STA, self.absolute_x(), Cycles::Fixed(5)),
            0x99 => (STA, self.absolute_y(), Cycles::Fixed(5)),
            0x81 => (STA, self.indirect_zero_page_x(), Cycles::Fixed(6)),
            0x91 => (STA, self.indirect_zero_page_y(), Cycles::Fixed(6)),

            0x86 => (STX, self.zero_page(), Cycles::Fixed(3)),
            0x96 => (STX, self.zero_page_y(), Cycles::Fixed(4)),
            0x8E => (STX, self.absolute(), Cycles::Fixed(4)),

            0x84 => (STY, self.zero_page(), Cycles::Fixed(3)),
            0x94 => (STY, self.zero_page_x(), Cycles::Fixed(4)),
            0x8C => (STY, self.absolute(), Cycles::Fixed(4)),

            0xAA => (TAX, self.implied(), Cycles::Fixed(2)),
            0xA8 => (TAY, self.implied(), Cycles::Fixed(2)),
            0xBA => (TSX, self.implied(), Cycles::Fixed(2)),
            0x8A => (TXA, self.implied(), Cycles::Fixed(2)),
            0x9A => (TXS, self.implied(), Cycles::Fixed(2)),
            0x98 => (TYA, self.implied(), Cycles::Fixed(2)),

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

    fn check_and_set_overflow(&mut self, x: u8, y: u8, sum: u8) {
        let s_x = x as i8;
        let s_y = y as i8;
        let s_sum = sum as i8;
        self.processor_status = if s_x >= 0 && s_y >= 0 && s_sum < 0 {
            self.processor_status.set_overflow()
        } else if s_x <= 0 && s_y <= 0 && s_sum > 0 {
            self.processor_status.set_overflow()
        } else {
            self.processor_status.clear_overflow()
        };
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

    // Returns whether or not a page was crossed.
    fn branch_offset(&mut self, offset: u8) -> PageBoundaryResult {
        // Treat the offset as signed and add it to the PC directly.
        let new_addr = self.pc.wrapping_add_signed((offset as i8).into());
        let page_crossed = new_addr & 0xFF00 != self.pc & 0xFF00;
        self.pc = new_addr;
        if page_crossed { PageBoundaryResult::PageBoundaryCrossed } else { PageBoundaryResult::SamePage }
    }

    // Add With Carry
    // A <- A + M + C
    // Affects Flags: N, V, Z, C
    fn adc(&mut self, m: u8) {
        let extended_result = (self.a as u16) + (m as u16) + (self.processor_status.carry() as u16);
        let c = (extended_result >> 8) & 0x0001 == 0x0001;

        let result = extended_result as u8;

        // Flags
        self.check_and_set_negative(result);
        self.check_and_set_overflow(self.a, m, result);
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
    fn bcc(&mut self, m: u8) -> (bool, PageBoundaryResult) {
        if !self.processor_status.is_carry() {
            (true, self.branch_offset(m))
        } else {
            (false, PageBoundaryResult::Irrelevant)
        }
    }

    // Branch on Carry Set
    // branch on C = 1
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bcs(&mut self, m: u8) -> (bool, PageBoundaryResult) {
        if self.processor_status.is_carry() {
            (true, self.branch_offset(m))
        } else {
            (false, PageBoundaryResult::Irrelevant)
        }
    }

    // Branch on Zero Flag Set
    // branch on Z = 1
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn beq(&mut self, m: u8) -> (bool, PageBoundaryResult) {
        if self.processor_status.is_zero() {
            (true, self.branch_offset(m))
        } else {
            (false, PageBoundaryResult::Irrelevant)
        }
    }

    // Bit Test
    // Sets the zero flag based on the value of A & M.
    // Also sets N and V based on bits 7 and 6 of M respectively.
    // Affects Flags: N V Z
    // Returns a bool for whether or not we branch.
    fn bit(&mut self, m: u8) {
        self.check_and_set_negative(m);
        if (m >> 6) & 0x01 == 0x01 {
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
    fn bmi(&mut self, m: u8) -> (bool, PageBoundaryResult) {
        if self.processor_status.is_negative() {
            (true, self.branch_offset(m))
        } else {
            (false, PageBoundaryResult::Irrelevant)
        }
    }

    // Branch on Result Not Zero
    // branch on Z = 0
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bne(&mut self, m: u8) -> (bool, PageBoundaryResult) {
        if !self.processor_status.is_zero() {
            (true, self.branch_offset(m))
        } else {
            (false, PageBoundaryResult::Irrelevant)
        }
    }

    // Branch on Result Plus
    // branch on N = 0
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bpl(&mut self, m: u8) -> (bool, PageBoundaryResult) {
        if !self.processor_status.is_negative() {
            (true, self.branch_offset(m))
        } else {
            (false, PageBoundaryResult::Irrelevant)
        }
    }

    // Forces a software interrupt. We do the following:
    // 1. Set the interrupt flag,
    // 2. Push the PC+1 to the stack (return address is 1 byte after the current PC because the BRK is always followed by a dummy opcode)
    // 3. Push the status register to the stack
    fn brk(&mut self) {
        self.processor_status = self.processor_status.set_break();
        let return_address = self.pc.wrapping_add(1);
        // Little Endian, push $HH, then $LL.
        self.push_stack((return_address >> 8) as u8);
        self.push_stack(return_address as u8);
        self.push_stack(self.processor_status.into());
    }

    // Branch on Overflow Clear
    // branch on V = 0
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bvc(&mut self, m: u8) -> (bool, PageBoundaryResult) {
        if !self.processor_status.is_overflow() {
            (true, self.branch_offset(m))
        } else {
            (false, PageBoundaryResult::Irrelevant)
        }
    }

    // Branch on Overflow Set
    // branch on V = 1
    // Affects Flags: (none)
    // Returns a bool for whether or not we branch.
    fn bvs(&mut self, m: u8) -> (bool, PageBoundaryResult) {
        if self.processor_status.is_overflow() {
            (true, self.branch_offset(m))
        } else {
            (false, PageBoundaryResult::Irrelevant)
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

    // Helper method for the cmp ops. Subtracts b from a and then:
    // 1. Sets carry flag if a >= b (unsigned)
    // 2. Sets negative if the subtraction is negative
    // 3. Sets zero if the subtraction is 0.
    fn cmp_processor_status(&mut self, a: u8, b: u8) {
        let subtraction = a.wrapping_sub(b);
        if a >= b {
            self.processor_status = self.processor_status.set_carry()
        } else {
            self.processor_status = self.processor_status.clear_carry();
        };

        if (subtraction & 0x80) == 0x80 {
            self.processor_status = self.processor_status.set_negative();
        } else {
            self.processor_status = self.processor_status.clear_negative();
        };

        if subtraction == 0 {
            self.processor_status = self.processor_status.set_zero();
        } else {
            self.processor_status = self.processor_status.clear_zero();
        };
    }

    // Compare accumulator to memory (this *only* sets flags based on the value of A - M).
    // This will set the carry if A - M > 0, the negative flag if A - M < 0, and the zero flag if A - M = 0.
    // A - M
    // Affects Flags: N Z C
    fn cmp(&mut self, m: u8) {
        self.cmp_processor_status(self.a, m);
    }

    // Compare X register to memory (this *only* sets flags based on the value of X - M).
    // This will set the carry if X - M > 0, the negative flag if X - M < 0, and the zero flag if X - M = 0.
    // X - M
    // Affects Flags: N Z C
    fn cpx(&mut self, m: u8) {
        self.cmp_processor_status(self.x, m);
    }

    // Compare accumulator to memory (this *only* sets flags based on the value of Y - M).
    // This will set the carry if Y - M > 0, the negative flag if Y - M < 0, and the zero flag if Y - M = 0.
    // Y - M
    // Affects Flags: N Z C
    fn cpy(&mut self, m: u8) {
        self.cmp_processor_status(self.y, m);
    }

    // Decrement memory by one. Requires us to *write* to a location in memory.
    // M <- M - 1
    // Affects Flags: N Z
    fn dec(&mut self, m: u8, address: u16) {
        let result = m.wrapping_sub(1);
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
    fn eor(&mut self, m: u8) {
        self.a = self.a ^ m;
        self.check_and_set_negative(self.a);
        self.check_and_set_zero(self.a);
    }
    
    // Increment memory by one. Requires us to *write* to a location in memory.
    // M <- M + 1
    // Affects Flags: N Z
    fn inc(&mut self, m: u8, address: u16) {
        let result = m.wrapping_add(1);
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

    // Jump to a new location and save the return address - 1 on the stack. Since we're already in the next byte we
    // need to decrement the pc once.
    // push return address (PC - 1)
    // PC <- $HHLL
    // Affects Flags: (none)
    fn jsr(&mut self, address: u16) {
        // Little Endian: push the $HH, then $LL so that RTI can pull properly.
        let return_address = self.pc.wrapping_sub(1);
        self.push_stack((return_address >> 8) as u8);
        self.push_stack(return_address as u8);
        self.pc = address;
    }

    // Load accumulator with data from memory.
    // A <- M
    // Affects Flags: N Z
    fn lda(&mut self, m: u8) {
        self.check_and_set_negative(m);
        self.check_and_set_zero(m);
        self.a = m;
    }

    // Load register X with data from memory.
    // X <- M
    // Affects Flags: N Z
    fn ldx(&mut self, m: u8) {
        self.check_and_set_negative(m);
        self.check_and_set_zero(m);
        self.x = m;
    }

    // Load accumulator with data from memory.
    // A <- M
    // Affects Flags: N Z
    fn ldy(&mut self, m: u8) {
        self.check_and_set_negative(m);
        self.check_and_set_zero(m);
        self.y = m;
    }

    // Logical Shift Right for accumulator or memory location.
    // Original 0 bit is shifted into carry and bit 7 is always 0, so N is cleared.
    // Affects Flags: N=0 Z C
    fn lsr(&mut self, m: u8, write_location: WriteLocation) {
        let result = m >> 1;

        // Flags
        self.processor_status = self.processor_status.clear_negative();
        self.check_and_set_zero(result);
        self.check_and_set_carry(m & 0x01 == 0x01);

        match write_location {
            WriteLocation::Accumulator => {
                self.a = result
            },
            WriteLocation::Memory(address) => {
                self.memory.write_byte(address, result);
            }
        }
    }
    
    // OR A with M.
    // A <- A OR M
    // Affects flags: N Z
    fn ora(&mut self, m: u8) {
        self.a = self.a | m;
        self.check_and_set_negative(self.a);
        self.check_and_set_zero(self.a);
    }

    // Push the accumulator onto the stack
    // Affects Flags: (none)
    fn pha(&mut self) {
        self.push_stack(self.a);
    }

    // Push the processor status onto the stack
    // Affects Flags: (none)
    fn php(&mut self) {
        self.push_stack(self.processor_status.set_break().set_bit_five().into());
    }

    // Pulls the accumulator from the stack
    // Affects Flags: N Z
    fn pla(&mut self) {
        let a = self.pull_stack();
        self.check_and_set_negative(a);
        self.check_and_set_zero(a);
        self.a = a;
    }

    // Pulls the processor status from the stack, ignoring the break flag.
    // Affects Flags: (entirely from the stack)
    fn plp(&mut self) {
        self.processor_status = ProcessorStatus::from(self.pull_stack()).clear_break();
    }

    // Rotate bits left for accumulator or memory location.
    // Original 7 bit is shifted into carry and carry is shifted into bit 0.
    // Affects Flags: N Z C
    fn rol(&mut self, m: u8, write_location: WriteLocation) {
        let result = (m << 1) + self.processor_status.carry();

        // Flags
        self.check_and_set_negative(result);
        self.check_and_set_zero(result);
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

    // Rotate bits right for accumulator or memory location.
    // Original 0 bit is shifted into carry and carry is shifted into bit 7.
    // Affects Flags: N Z C
    fn ror(&mut self, m: u8, write_location: WriteLocation) {
        let result = (m >> 1) + (self.processor_status.carry() << 7);

        // Flags
        self.check_and_set_negative(result);
        self.check_and_set_zero(result);
        self.check_and_set_carry(m & 0x01 == 0x01);

        match write_location {
            WriteLocation::Accumulator => {
                self.a = result
            },
            WriteLocation::Memory(address) => {
                self.memory.write_byte(address, result);
            }
        }
    }

    // Return from Interrupt
    // Pull SR, pull PC
    // Affects Flags: (whatever you pull for status register)
    fn rti(&mut self) {
        self.processor_status = ProcessorStatus::from(self.pull_stack()).clear_break();
        self.pc = self.pull_address();
    }

    // Return from Subroutine
    // pull PC (but it's stored as PC - 1, so increment it once).
    // Affects Flags: (none)
    fn rts(&mut self) {
        self.pc = self.pull_address().wrapping_add(1);
    }

    // Subtract Memory from Accumulator With Borrow
    // A <- A - M - !C
    // Affects Flags: N Z C V
    fn sbc(&mut self, m: u8) {
        let extended_result = (self.a as u16) + (m.wrapping_neg() as u16) + if !self.processor_status.is_carry() { 0x01_u8.wrapping_neg() as u16 } else { 0x0000 };
        let c = (extended_result >> 8) & 0x0001 == 0x0001;

        let result = extended_result as u8;

        // Flags
        self.check_and_set_negative(result);
        self.check_and_set_overflow(self.a, m.wrapping_neg(),result);
        self.check_and_set_zero(result);
        self.check_and_set_carry(c);

        self.a = result as u8;
    }

    // Set the carry flag.
    fn sec(&mut self) {
        self.processor_status = self.processor_status.set_carry();
    }

    // Set the decimal flag.
    fn sed(&mut self) {
        self.processor_status = self.processor_status.set_decimal();
    }

    // Set the interrupt disable flag.
    fn sei(&mut self) {
        self.processor_status = self.processor_status.set_interrupt();
    }

    // Store accumulator into memory
    // M <- A
    // Affects flags: (none)
    fn sta(&mut self, address: u16) {
        self.memory.write_byte(address, self.a);
    }

    // Store X into memory
    // M <- X
    // Affects flags: (none)
    fn stx(&mut self, address: u16) {
        self.memory.write_byte(address, self.x);
    }

    // Store Y into memory
    // M <- Y
    // Affects flags: (none)
    fn sty(&mut self, address: u16) {
        self.memory.write_byte(address, self.y);
    }

    // Transfer accumulator to X
    // X <- A
    // Affects flags: N Z
    fn tax(&mut self) {
        self.x = self.a;
        self.check_and_set_negative(self.x);
        self.check_and_set_zero(self.x);
    }

    // Transfer accumulator to Y
    // Y <- A
    // Affects flags: N Z
    fn tay(&mut self) {
        self.y = self.a;
        self.check_and_set_negative(self.y);
        self.check_and_set_zero(self.y);
    }

    // Transfer stack pointer to X
    // X <- SP
    // Affects flags: N Z
    fn tsx(&mut self) {
        self.x = self.sp;
        self.check_and_set_negative(self.x);
        self.check_and_set_zero(self.x);
    }

    // Transfer X to A
    // A <- X
    // Affects flags: N Z
    fn txa(&mut self) {
        self.a = self.x;
        self.check_and_set_negative(self.a);
        self.check_and_set_zero(self.a);
    }

    // Transfer X to stack pointer
    // SP <- X
    // Affects flags: (none)
    fn txs(&mut self) {
        self.sp = self.x;
    }

    // Transfer Y to A
    // A <- Y
    // Affects flags: N Z
    fn tya(&mut self) {
        self.a = self.y;
        self.check_and_set_negative(self.a);
        self.check_and_set_zero(self.a);
    }

    // ----------- Interrupt Handling ------------- //
    // According to the docs:
    // 1. Recognize interrupt request has occurred.
    // 2. Complete execution of the current instruction.
    // 3. Push the program counter and status register on to the stack.
    // 4. Set the interrupt disable flag to prevent further interrupts.
    // 5. Load the address of the interrupt handling routine from the vector table into the program
    // counter.
    // 6. Execute the interrupt handling routine.
    // 7. After executing a RTI (Return From Interrupt) instruction, pull the program counter and
    // status register values from the stack.
    // 8. Resume execution of the program. 
    fn handle_nmi_interrupt(&mut self) {
        // 1
        if !self.is_nmi() {
            return;
        }
        self.clear_nmi();

        // 2 this is called after the current instruction.

        // 3
        self.push_pc();
        self.push_stack(self.processor_status.into());

        // 4
        self.processor_status = self.processor_status.set_interrupt();

        // 5
        self.pc = self.memory.read_two_bytes(NMI_ADDRESS);

        // Steps 6, 7, 8 will be done automatically.
    }

    // ----------- Instruction Fetching ----------- //

    // A bit of a hack to deal with the variability of branch cycles.
    fn calculate_branch_cycles(num_cycles: &mut usize, branch_and_pbr: (bool, PageBoundaryResult)) {
        // Cycle Calculation
        // Branch | PBR | Cycles
        //    F   |  F  | 2
        //    F   |  T  | 2
        //    T   |  F  | 3
        //    T   |  T  | 4
        *num_cycles = match branch_and_pbr {
            (true, PageBoundaryResult::PageBoundaryCrossed) => 4,
            (true, _) => 3,
            (false, _) => 2,
        };
    }

    // Attempts to execute the cycles for one instruction.
    pub fn execute_cycles_for_one_instruction(&mut self) -> bool {
        self.cycle_budget += 1;
        if self.cycle_budget < 0 {
            return false;
        } 

        let num_cycles = self.fetch_instruction_and_execute() as i8;
        self.dec_nmi(num_cycles as u8);
        self.cycle_budget -= num_cycles;
        // Check if we have an interrupt (NMI) enabled.
        self.handle_nmi_interrupt();
        true
    }

    pub fn fetch_instruction_and_execute(&mut self) -> usize {
        let FetchInstructionResult { opcode, addressing_mode, cycles } = self.fetch_instruction();

        // Now our PC is at the next instruction, so offsets will be measured relative to that.
        let AddressingModeData { data, address, page_boundary_result } = addressing_mode.into_data(self);
        let pbc = page_boundary_result == PageBoundaryCrossed;
        let mut num_cycles = match (cycles, pbc) {
            (Cycles::Fixed(n), _) => n,
            (Cycles::PageCrossing(n), false) => n,
            (Cycles::PageCrossing(n), true) => n + 1,
        };
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
            Opcode::BCC => Self::calculate_branch_cycles(&mut num_cycles, self.bcc(data)),
            Opcode::BCS => Self::calculate_branch_cycles(&mut num_cycles, self.bcs(data)),
            Opcode::BEQ => Self::calculate_branch_cycles(&mut num_cycles, self.beq(data)),
            Opcode::BIT => self.bit(data),
            Opcode::BMI => Self::calculate_branch_cycles(&mut num_cycles, self.bmi(data)),
            Opcode::BNE => Self::calculate_branch_cycles(&mut num_cycles, self.bne(data)),
            Opcode::BPL => Self::calculate_branch_cycles(&mut num_cycles, self.bpl(data)),
            Opcode::BRK => self.brk(),
            Opcode::BVC => Self::calculate_branch_cycles(&mut num_cycles, self.bvc(data)),
            Opcode::BVS => Self::calculate_branch_cycles(&mut num_cycles, self.bvs(data)),
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
            Opcode::INC => self.inc(data, address.expect("Address should be supplied for a INC!")),
            Opcode::INX => self.inx(),
            Opcode::INY => self.iny(),
            Opcode::JMP => self.jmp(address.expect("Address should have been supplied for a JMP!")),
            Opcode::JSR => self.jsr(address.expect("Address should have been supplied for a JSR!")),
            Opcode::LDA => self.lda(data),
            Opcode::LDX => self.ldx(data),
            Opcode::LDY => self.ldy(data),
            Opcode::LSR => {
                // We write to memory if we returned a specific address.
                let (data, wl) = if let Some(address) = address {
                    (data, WriteLocation::Memory(address))
                } else {
                    (self.a, WriteLocation::Accumulator)
                };
                self.lsr(data, wl);
            },
            Opcode::NOP => {}, // an actual noop
            Opcode::ORA => self.ora(data),
            Opcode::PHA => self.pha(),
            Opcode::PHP => self.php(),
            Opcode::PLA => self.pla(),
            Opcode::PLP => self.plp(),
            Opcode::ROL => {
                // We write to memory if we returned a specific address.
                let (data, wl) = if let Some(address) = address {
                    (data, WriteLocation::Memory(address))
                } else {
                    (self.a, WriteLocation::Accumulator)
                };
                self.rol(data, wl);
            },
            Opcode::ROR => {
                // We write to memory if we returned a specific address.
                let (data, wl) = if let Some(address) = address {
                    (data, WriteLocation::Memory(address))
                } else {
                    (self.a, WriteLocation::Accumulator)
                };
                self.ror(data, wl);
            },
            Opcode::RTI => self.rti(),
            Opcode::RTS => self.rts(),
            Opcode::SBC => self.sbc(data),
            Opcode::SEC => self.sec(),
            Opcode::SED => self.sed(),
            Opcode::SEI => self.sei(),
            Opcode::STA => self.sta(address.expect("Address should have been supplied for a STA!")),
            Opcode::STX => self.stx(address.expect("Address should have been supplied for a STX!")),
            Opcode::STY => self.sty(address.expect("Address should have been supplied for a STY!")),
            Opcode::TAX => self.tax(),
            Opcode::TAY => self.tay(),
            Opcode::TSX => self.tsx(),
            Opcode::TXA => self.txa(),
            Opcode::TXS => self.txs(),
            Opcode::TYA => self.tya(),
        }

        self.cycle_count += num_cycles;
        num_cycles
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
        assert_eq!(0xFA, cpu.sp);
        assert_eq!(0x00, cpu.memory.read_byte(0x01FD));
        assert_eq!(0x02, cpu.memory.read_byte(0x01FC));
        assert_eq!(0x72, cpu.memory.read_byte(0x01FB));
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
        assert!(cpu.processor_status.is_carry());

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
        cpu.fetch_instruction_and_execute();

        assert_eq!(cpu.pc, 0x1234);
        assert_eq!(5, cpu.cycle_count);
    }


    #[test]
    fn test_jsr() {
        let mut cpu = Cpu::initialize();
        cpu.pc = 0x1234;
        cpu.memory.write_bytes(0x1234, &[0x20, 0xEF, 0xBE]);

        cpu.fetch_instruction_and_execute();
        assert_eq!(cpu.pc, 0xBEEF);
        assert_eq!(cpu.cycle_count, 6);
        // Reading directly from the stack
        assert_eq!(0x12, cpu.memory.read_byte(0x01FD));
        assert_eq!(0x36, cpu.memory.read_byte(0x01FC));
    }

    #[test]
    fn test_loads() {
        let mut cpu = Cpu::initialize();
        cpu.memory.write_bytes(0x00, &[0xA9, 0x42]);

        cpu.fetch_instruction_and_execute();
        assert_eq!(cpu.pc, 0x02);
        assert_eq!(cpu.cycle_count, 2);
        assert_eq!(cpu.a, 0x42);        
    }

    #[test]
    fn test_lsr() {
        let mut cpu = Cpu::initialize();
        cpu.a = 0xF3; // 1 1 1 1 0 0 1 1
        cpu.memory.write_bytes(0x00, &[0x4A, 0x46, 0x42]);
        cpu.memory.write_byte(0x0042, 0x01);

        // One instruction should just right shift A and set the carry.
        cpu.fetch_instruction_and_execute();
        // 1 1 1 1 0 0 1 1 >> 1 = 0 1 1 1 1 0 0 1 = 0x79

        assert_eq!(0x01, cpu.pc);
        assert_eq!(0x79, cpu.a);
        assert!(!cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
        assert!(cpu.processor_status.is_carry());

        // Now we'll right shift a value directly on the zero page at 0x42.
        cpu.fetch_instruction_and_execute();

        assert_eq!(0x03, cpu.pc);
        assert_eq!(0x79, cpu.a);
        assert_eq!(0x00, cpu.memory.read_byte(0x0042));
        assert!(!cpu.processor_status.is_negative());
        assert!(cpu.processor_status.is_zero());
        assert!(cpu.processor_status.is_carry());
    }

    #[test]
    fn test_ora_and_eor() {
        let mut cpu = Cpu::initialize();
        cpu.a = 0x50;
        cpu.memory.write_bytes(0x00, &[0x09, 0x05, 0x49, 0xAA]);

        // One instruction should just OR A with 0x05.
        cpu.fetch_instruction_and_execute();
        assert_eq!(cpu.a, 0x55);
        assert!(!cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());

        // Another will effectively make A = 0xFF.
        cpu.fetch_instruction_and_execute();
        assert_eq!(cpu.a, 0xFF);
        assert!(cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
    }

    #[test]
    fn test_rol_and_ror() {
        let mut cpu = Cpu::initialize();
        cpu.a = 0xF3; // 1 1 1 1 0 0 1 1
        cpu.memory.write_bytes(0x00, &[0x6A, 0x26, 0x42]);
        cpu.memory.write_byte(0x0042, 0x7F);

        // One instruction should just rotate A to the right and set the carry.
        cpu.fetch_instruction_and_execute();
        // 1 1 1 1 0 0 1 1 >> 1 = 0 1 1 1 1 0 0 1 = 0x79

        assert_eq!(0x01, cpu.pc);
        assert_eq!(0x79, cpu.a);
        assert!(!cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
        assert!(cpu.processor_status.is_carry());

        // Now we'll left rotate a value directly on the zero page at 0x42. The carry is set!
        // 0 1 1 1 1 1 1 1 << 1 + 1
        cpu.fetch_instruction_and_execute();

        assert_eq!(0x03, cpu.pc);
        assert_eq!(0x79, cpu.a);
        assert_eq!(0xFF, cpu.memory.read_byte(0x0042));
        assert!(cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
        assert!(!cpu.processor_status.is_carry());
    }

    #[test]
    fn test_rti() {
        let mut cpu = Cpu::initialize();
        cpu.memory.write_byte(0x00, 0x40);
        // Write to the stack directly like a maniac (as if we had done a BRK).
        cpu.push_stack(0x12);
        cpu.push_stack(0x34);
        cpu.push_stack(0xD8); // 1 1 0 1 1 0 0 0

        cpu.fetch_instruction_and_execute();

        assert_eq!(cpu.processor_status.into(), 0xE8); // Break is cleared
        assert_eq!(cpu.pc, 0x1234);
    }

    #[test]
    fn test_rts() {
        let mut cpu = Cpu::initialize();
        // Jump to a subroutine where we load X, then come back and load the accumulator.
        // JSR #0x1234; LDA #0x42;
        cpu.memory.write_bytes(0x00, &[0x20, 0x34, 0x12, 0xA9, 0x42]);
        // LDX, #0xFF; RTI;
        cpu.memory.write_bytes(0x1234, &[0xA2, 0xFF, 0x60]);

        // Execute the four instructions.
        cpu.fetch_instruction_and_execute();
        cpu.fetch_instruction_and_execute();
        cpu.fetch_instruction_and_execute();
        cpu.fetch_instruction_and_execute();

        assert_eq!(cpu.pc, 0x05);
        assert_eq!(cpu.a, 0x42);
        assert_eq!(cpu.x, 0xFF);
        // Cycles for a JSR, LDA, RTI, and LDA
        assert_eq!(cpu.cycle_count, 6 + 2 + 6 + 2);
    }

    #[test]
    fn test_sbc() {
        let mut cpu = Cpu::initialize();
        cpu.a = 0x03F;
        cpu.memory.write_bytes(0x00, &[0xE9, 0x02]);
        cpu.fetch_instruction_and_execute();

        assert_eq!(0x3C, cpu.a);
        assert_eq!(0x02, cpu.pc);
        assert!(!cpu.processor_status.is_overflow());
        assert!(!cpu.processor_status.is_negative());
        assert!(!cpu.processor_status.is_zero());
        assert!(!cpu.processor_status.is_carry());
    }

    #[test]
    fn test_sta() {
        let mut cpu = Cpu::initialize();
        cpu.a = 0x42;
        cpu.memory.write_bytes(0x00, &[0x8D, 0x34, 0x12]);

        cpu.fetch_instruction_and_execute();
        assert_eq!(cpu.memory.read_byte(0x1234), 0x42);
    }

    #[test]
    fn test_tax() {
        let mut cpu = Cpu::initialize();
        cpu.a = 0xF2;
        cpu.memory.write_bytes(0x00, &[0xAA, 0x9A]);

        cpu.fetch_instruction_and_execute();
        cpu.fetch_instruction_and_execute();

        assert_eq!(cpu.x, 0xF2);
        assert_eq!(cpu.sp, 0xF2);
    }
}
