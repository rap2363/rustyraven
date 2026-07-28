#[derive(Clone, Copy, PartialEq)]
enum Button {
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

pub enum ButtonReader {
    Keyboard,
    Tester(Vec<Button>),
}

impl ButtonReader {
    fn read_keyboard_button(&self, button: Button) -> bool {
        match button {
            _ => todo!("Implement me"),
        }
    }

    fn read_button(&self, button: Button) -> bool {
        match self {
            ButtonReader::Keyboard => self.read_keyboard_button(button),
            ButtonReader::Tester(buttons) => buttons.contains(&button),
        }
    }
}

pub struct Controller {
    strobe: bool,
    button: Button,
    button_reader: ButtonReader,
}

impl Controller {
    pub fn initialize(button_reader: ButtonReader) -> Self {
        Self {
            strobe: false,
            button: Button::A,
            button_reader: button_reader,
        }
    }

    pub fn strobe_high(&mut self) {
        self.strobe = true;
        self.button = Button::A;
    }

    pub fn strobe_low(&mut self) {
        self.strobe = false;
    }

    pub fn read(&mut self) -> bool {
        let data = self.button_reader.read_button(self.button);
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
                Button::None => Button::None,
            }
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_button_presses() {
        let mut controller = Controller::initialize(ButtonReader::Tester(vec![Button::B, Button::Down, Button::Start]));
        assert_eq!(controller.read(), false);
        controller.strobe_high();
        assert_eq!(controller.read(), false);
        controller.strobe_low();
        // Now subsequent reads will use the filled out button states.
        assert_eq!(controller.read(), false); // A
        assert_eq!(controller.read(), true); // B
        assert_eq!(controller.read(), false); // Select
        assert_eq!(controller.read(), true); // Start
        assert_eq!(controller.read(), false); // Up
        assert_eq!(controller.read(), true); // Down
        assert_eq!(controller.read(), false); // Left
        assert_eq!(controller.read(), false); // Right
        assert_eq!(controller.read(), false); // None
    }
}