use crate::processor_status::ProcessorStatus;
use crate::cpu::Cpu;

#[derive(Debug)]
pub enum Opcode {
    ADC,
}

impl Opcode {
    // Returns what the new value of a should be along with the updated ProcessorStatus
    fn adc_execute(a: u8, m: u8, status: ProcessorStatus) -> (u8, ProcessorStatus) {
        let (a, carry) = a.carrying_add(m, status.is_carry());
        let s_a = a as i8;
        let s_m = m as i8;
        let mut new_status = status;
        if s_a < 0 {
            new_status = new_status.set_negative();
        }
        if a == 0 {
            new_status = new_status.set_zero();
        }
        if carry {
            new_status = new_status.set_carry();
        }
        let both_positive = s_a > 0 && s_m > 0;
        let both_negative = s_a < 0 && s_m < 0;
        if both_positive && (((a + m + status.carry()) as i8) < 0) {
            new_status.set_overflow();
        }
        if both_negative && (((a + m + status.carry()) as i8) > 0) {
            new_status.set_overflow();
        }

        (a, new_status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_with_carry() {
        let x = Opcode::ADC;
        // let result = x.execute(ProcessorStatus::initialize());
        // println!("{:?}", result);
        assert_eq!(1, 2);
    }
}