#[derive(Debug, Default)]
struct StatusFlags {
    n: bool,
    z: bool,
    c: bool,
    i: bool,
    d: bool,
    v: bool,
}

struct StatusFlagsBuilder {
    inner: StatusFlags,
}

impl StatusFlagsBuilder {
    fn set_negative(mut self) -> Self {
        self.inner.n = true;
        self
    }

    fn clear_negative(mut self) -> Self {
        self.inner.n = false;
        self
    }

    fn set_zero(mut self) -> Self {
        self.inner.z = true;
        self
    }

    fn clear_zero(mut self) -> Self {
        self.inner.z = false;
        self
    }

    fn set_carry(mut self) -> Self {
        self.inner.c = true;
        self
    }

    fn clear_carry(mut self) -> Self {
        self.inner.c = false;
        self
    }

    fn set_interrupt(mut self) -> Self {
        self.inner.i = true;
        self
    }

    fn clear_interrupt(mut self) -> Self {
        self.inner.i = false;
        self
    }

    fn set_overflow(mut self) -> Self {
        self.inner.v = true;
        self
    }

    fn clear_overflow(mut self) -> Self {
        self.inner.v = false;
        self
    }

    fn build(self) -> StatusFlags {
        StatusFlags {
            n: self.inner.n,
            z: self.inner.z,
            c: self.inner.c,
            i: self.inner.i,
            d: self.inner.d,
            v: self.inner.v,
        }
    }
}

impl StatusFlags {
    fn new() -> StatusFlagsBuilder {
        StatusFlagsBuilder {
            inner: StatusFlags::default()
        }
    }

    fn from(status_flags: Self) -> StatusFlagsBuilder {
        StatusFlagsBuilder {
            inner: status_flags
        }
    }
}

#[derive(Debug)]
enum Opcode {
    ADC,
}

#[derive(Debug)]
struct OpcodeResult {
    num_cycles: usize,
    status: StatusFlags,
}

impl Opcode {
    fn execute(&self) -> OpcodeResult {
        match self {
            Opcode::ADC => Self::adc_execute(0, 0, false, StatusFlags::default()),
        }
    }

    fn adc_execute(a: u8, m: u8, c: bool, status: StatusFlags) -> OpcodeResult {
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