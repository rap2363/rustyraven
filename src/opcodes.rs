use crate::processor_status::ProcessorStatus;

#[derive(Debug)]
enum Opcode {
    ADC,
}

#[derive(Debug)]
struct OpcodeResult {
    num_cycles: usize,
    status: ProcessorStatus,
}

impl Opcode {
    fn execute(&self) -> OpcodeResult {
        match self {
            Opcode::ADC => Self::adc_execute(0, 0, false, ProcessorStatus::initialize()),
        }
    }

    fn adc_execute(a: u8, m: u8, c: bool, status: ProcessorStatus) -> OpcodeResult {
        OpcodeResult {
            num_cycles: 0,
            status: status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_with_carry() {
        let x = Opcode::ADC;
        let result = x.execute();
        println!("{:?}", result);
        assert_eq!(1, 2);
    }
}