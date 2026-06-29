// Represents a static, contiguous layout of memory (in bytes) and offers
// low-level API's for reading and writing. Multiple segments are used
// to build up main memory (RAM) for the CPU.
struct Segment<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> Segment<N> {
    // Initializes the segment to all zeros.
    pub fn initialize() -> Self {
        Self {
            data: [0; N],
        }
    }

    pub fn set_byte(&mut self, address: usize, value: u8) {
        self.data[address] = value;
    }

    pub fn get_byte(&self, address: usize) -> u8 {
        self.data[address]
    }
}

pub struct Memory {
    ram: Segment<0x0800>,
    lower_io: Segment<0x0008>,
    upper_memory: Segment<0xC000>,
}

impl Memory {
    pub fn initialize() -> Self {
        Self {
            ram: Segment::<0x0800>::initialize(),
            lower_io: Segment::<0x0008>::initialize(),
            upper_memory: Segment::<0xC000>::initialize(),
        }
    }

    pub fn set_byte(&mut self, address: u16, value: u8) {
        if address < 0x2000 {
            // RAM
            let ram_address = address % 0x0800;
            self.ram.set_byte(ram_address as usize, value);
        } else if address < 0x4000 {
            // Lower I/O
            let lower_io_address = (address - 0x2000) % 0x0008;
            println!("{}", lower_io_address);
            self.lower_io.set_byte(lower_io_address as usize, value);
        } else {
            // Upper Memory
            let upper_memory_address = address - 0x4000;
            self.upper_memory.set_byte(upper_memory_address as usize, value);
        }
    }

    pub fn get_byte(&self, address: u16) -> u8 {
        if address < 0x2000 {
            // RAM
            let ram_address = address % 0x0800;
            self.ram.get_byte(ram_address as usize)
        } else if address < 0x4000 {
            // Lower I/O
            let lower_io_address = (address - 0x2000) % 0x0008;
            self.lower_io.get_byte(lower_io_address as usize)
        } else {
            // Upper Memory
            let upper_memory_address = address - 0x4000;
            self.upper_memory.get_byte(upper_memory_address as usize)
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_16_byte_memory() {
        let mut memory = Segment::<16>::initialize();
        assert_eq!(memory.get_byte(3), 0);
        memory.set_byte(2, b'A');
        assert_eq!(memory.get_byte(2), b'A');
    }

    #[test]
    fn test_memory_mirroring() {
        let mut cpu_memory = Memory::initialize();
        cpu_memory.set_byte(0x0803, 42);
        cpu_memory.set_byte(0x2009, 43);
        // Assert that the write can be read in a "mirrored" way throughout RAM.
        assert_eq!(cpu_memory.get_byte(0x0003), 42);
        assert_eq!(cpu_memory.get_byte(0x0803), 42);
        assert_eq!(cpu_memory.get_byte(0x1003), 42);
        assert_eq!(cpu_memory.get_byte(0x1803), 42);
        // And lower I/O
        assert_eq!(cpu_memory.get_byte(0x2001), 43);
        assert_eq!(cpu_memory.get_byte(0x2009), 43);
        assert_eq!(cpu_memory.get_byte(0x2011), 43);
    }
}