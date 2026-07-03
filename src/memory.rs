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

    pub fn write_byte(&mut self, address: usize, value: u8) {
        self.data[address] = value;
    }

    pub fn read_byte(&self, address: usize) -> u8 {
        self.data[address]
    }
}

pub struct CpuMemory {
    ram: Segment<0x0800>,
    lower_io: Segment<0x0008>,
    upper_memory: Segment<0xC000>,
}

impl CpuMemory {
    pub fn initialize() -> Self {
        Self {
            ram: Segment::<0x0800>::initialize(),
            lower_io: Segment::<0x0008>::initialize(),
            upper_memory: Segment::<0xC000>::initialize(),
        }
    }

    pub fn write_byte_to_stack(&mut self, sp: u8, value: u8) {
        self.write_byte(0x1000 + (sp as u16), value);
    }

    pub fn read_byte_from_stack(&self, sp: u8) -> u8 {
        self.read_byte(0x1000 + (sp as u16))
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        if address < 0x2000 {
            // RAM
            let ram_address = address % 0x0800;
            self.ram.write_byte(ram_address as usize, value);
        } else if address < 0x4000 {
            // Lower I/O
            let lower_io_address = (address - 0x2000) % 0x0008;
            println!("{}", lower_io_address);
            self.lower_io.write_byte(lower_io_address as usize, value);
        } else {
            // Upper Memory
            let upper_memory_address = address - 0x4000;
            self.upper_memory.write_byte(upper_memory_address as usize, value);
        }
    }

    pub fn write_bytes(&mut self, address: u16, values: &[u8]) {
        for i in 0..values.len() {
            self.write_byte(address + (i as u16), values[i]);
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        if address < 0x2000 {
            // RAM
            let ram_address = address % 0x0800;
            self.ram.read_byte(ram_address as usize)
        } else if address < 0x4000 {
            // Lower I/O
            let lower_io_address = (address - 0x2000) % 0x0008;
            self.lower_io.read_byte(lower_io_address as usize)
        } else {
            // Upper Memory
            let upper_memory_address = address - 0x4000;
            self.upper_memory.read_byte(upper_memory_address as usize)
        }
    }

    pub fn read_byte_zero_page(&self, address: u8) -> u8 {
        // This is obviously within the RAM memory segment.
        self.ram.read_byte(address as usize)
    }

    // Returns two bytes assuming little endian. So the bytes
    // come back $HHLL even though they're *read* as LLHH.
    //
    // Note this wraps around the entire memory space!
    pub fn read_two_bytes(&self, address: u16) -> u16 {
        u16::from_le_bytes([
            self.read_byte(address), 
            self.read_byte(address.wrapping_add(1)),
        ])
    }

    // Returns two bytes assuming little endian. So the bytes
    // come back $HHLL even though they're *read* as LLHH. Add
    // wraps around the Zero Page.
    pub fn read_two_bytes_zero_page(&self, address: u8) -> u16 {
        u16::from_le_bytes([
            self.read_byte(address as u16),
            self.read_byte(address.wrapping_add(1) as u16), 
        ])
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_16_byte_memory() {
        let mut memory = Segment::<16>::initialize();
        assert_eq!(memory.read_byte(3), 0);
        memory.write_byte(2, b'A');
        assert_eq!(memory.read_byte(2), b'A');
    }

    #[test]
    fn test_memory_mirroring() {
        let mut cpu_memory = CpuMemory::initialize();
        cpu_memory.write_byte(0x0803, 42);
        cpu_memory.write_byte(0x2009, 43);
        // Assert that the write can be read in a "mirrored" way throughout RAM.
        assert_eq!(cpu_memory.read_byte(0x0003), 42);
        assert_eq!(cpu_memory.read_byte(0x0803), 42);
        assert_eq!(cpu_memory.read_byte(0x1003), 42);
        assert_eq!(cpu_memory.read_byte(0x1803), 42);
        // And lower I/O
        assert_eq!(cpu_memory.read_byte(0x2001), 43);
        assert_eq!(cpu_memory.read_byte(0x2009), 43);
        assert_eq!(cpu_memory.read_byte(0x2011), 43);
    }
}