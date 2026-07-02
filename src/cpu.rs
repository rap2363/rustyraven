use crate::addressing_modes::AddressingMode;
use crate::memory::CpuMemory;
use crate::processor_status::ProcessorStatus;
use crate::opcodes::Opcode;

pub struct Cpu {
    pub memory: CpuMemory,
    pub processor_status: ProcessorStatus,
    pub pc: u16,
    pub sp: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
}

pub struct InstructionFetch {
    opcode: Opcode,
    addressing_mode: AddressingMode,
    cycles: usize,
}

struct Cycles(usize);

impl InstructionFetch {
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
        }
    }

    fn increment_pc(&mut self) -> u16 {
        self.pc = self.pc.wrapping_add(1);
        self.pc
    }

    fn immediate(&mut self) -> AddressingMode {
        let pc = self.increment_pc();
        let data = self.memory.get_byte(pc);
        self.increment_pc();
        AddressingMode::Immediate(data)
    }

    fn zero_page(&mut self) -> AddressingMode {
        let pc = self.increment_pc();
        let address = self.memory.get_byte(pc);
        self.increment_pc();
        AddressingMode::ZeroPage(address)
    }

    fn zero_page_x(&mut self) -> AddressingMode {
        let pc = self.increment_pc();
        let address = self.memory.get_byte(pc);
        self.increment_pc();
        InstructionFetch::new(Opcode::ADC, AddressingMode::IndexedZeroPageX(address),4)
    }

    pub fn fetch_instruction(&mut self) -> InstructionFetch {
        let opcode_byte = self.memory.get_byte(self.pc);
        match opcode_byte {
            // ADC, Immediate
            0x69 => InstructionFetch::new(Opcode::ADC, self.immediate(), 2),
            // ADC, Zero Page
            0x65 => InstructionFetch::new(Opcode::ADC, self.zero_page(), 3),
            // ADC, Zero Page, X
            0x75 => InstructionFetch::new(Opcode::ADC, self.zero_page_x(), 4),
            // ADC, Absolute
            0x6D => {
                let pc = self.increment_pc();
                let address = self.memory.get_two_bytes(pc);
                InstructionFetch::new(Opcode::ADC, AddressingMode::Absolute(address),4)
            },
            // ADC, Absolute, X
            0x7D => {
                let pc = self.increment_pc();
                let address = self.memory.get_two_bytes(pc);
                InstructionFetch::new(Opcode::ADC, AddressingMode::IndexedX(address),4)
            },
            // ADC, Absolute, X
            0x79 => {
                let pc = self.increment_pc();
                let address = self.memory.get_two_bytes(pc);
                InstructionFetch::new(Opcode::ADC, AddressingMode::IndexedY(address),4)
            },
            // ADC, Indirect Zero Page, X
            0x61 => {
                let pc = self.increment_pc();
                let address = self.memory.get_byte(pc);
                InstructionFetch::new(Opcode::ADC, AddressingMode::IndirectZeroPageX(address),6)
            },
            // ADC, Indirect Zero Page, X
            0x71 => {
                let pc = self.increment_pc();
                let address = self.memory.get_byte(pc);
                InstructionFetch::new(Opcode::ADC, AddressingMode::IndirectZeroPageY(address),5)
            },
            x => todo!("Unimplemented opcode: {x}!"),
        }
    }
}