use crate::memory::Memory;

struct Cpu {
    memory: Memory,
    sp: u16,
    a: u8,
    x: u8,
    y: u8,

}