#[derive(Clone, Copy, PartialEq)]
pub enum Button {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
    None,
}

pub struct Controller {
    strobe: bool,
    button: Button,
    a: bool,
    b: bool,
    select: bool,
    start: bool,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl Controller {
    pub fn initialize() -> Self {
        Self {
            strobe: false,
            button: Button::A,
            a: false,
            b: false,
            select: false,
            start: false,
            up: false,
            down: false,
            left: false,
            right: false,
        }
    }

    pub fn strobe_high(&mut self) {
        self.strobe = true;
        self.button = Button::A;
    }

    pub fn strobe_low(&mut self) {
        self.strobe = false;
    }

    pub fn read(&mut self) -> u8 {
        let data = self.read_button(self.button);
        if !self.strobe {
            self.button = match self.button {
                Button::A => Button::B,
                Button::B => Button::Select,
                Button::Select => Button::Start,
                Button::Start => Button::Up,
                Button::Up => Button::Down,
                Button::Down => Button::Left,
                Button::Left => Button::Right,
                Button::Right => Button::None,
                Button::None => Button::A,
            }
        }
        if data { 0x01 } else { 0x00 }
    }

    pub fn read_button(&self, button: Button) -> bool {
        match button {
            Button::A => self.a,
            Button::B => self.b,
            Button::Select => self.select,
            Button::Start => self.start,
            Button::Up => self.up,
            Button::Down => self.down,
            Button::Left => self.left,
            Button::Right => self.right,
            Button::None => false,
        }
    }

    pub fn set_button(&mut self, button: Button) {
        match button {
            Button::A => { self.a = true },
            Button::B => { self.b = true },
            Button::Select => { self.select = true },
            Button::Start => { self.start = true },
            Button::Up => { self.up = true },
            Button::Down => { self.down = true },
            Button::Left => { self.left = true },
            Button::Right => { self.right = true },
            Button::None => {},  
        }
    }

    pub fn clear_button(&mut self, button: Button) {
        match button {
            Button::A => { self.a = false },
            Button::B => { self.b = false },
            Button::Select => { self.select = false },
            Button::Start => { self.start = false },
            Button::Up => { self.up = false },
            Button::Down => { self.down = false },
            Button::Left => { self.left = false },
            Button::Right => { self.right = false },
            Button::None => {},  
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_button_presses() {
        let mut controller = Controller::initialize();
        controller.set_button(Button::B);
        controller.set_button(Button::Down);
        controller.set_button(Button::Start);

        assert_eq!(controller.read(), 0x00);
        controller.strobe_high();
        assert_eq!(controller.read(), 0x00);
        controller.strobe_low();
        // Now subsequent reads will use the filled out button states.
        assert_eq!(controller.read(), 0x00); // A
        assert_eq!(controller.read(), 0x01); // B
        assert_eq!(controller.read(), 0x00); // Select
        assert_eq!(controller.read(), 0x01); // Start
        assert_eq!(controller.read(), 0x00); // Up
        assert_eq!(controller.read(), 0x01); // Down
        assert_eq!(controller.read(), 0x00); // Left
        assert_eq!(controller.read(), 0x00); // Right
        assert_eq!(controller.read(), 0x00); // None
    }
}