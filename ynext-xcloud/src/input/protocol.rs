use bytes::{BufMut, Bytes, BytesMut};
use gilrs::{Axis, Button, Gamepad};

/// Representa o estado atual de um controle (estilo Xbox/XInput)
#[derive(Debug, Clone, Default)]
pub struct InputReport {
    pub gamepad_index: u8,
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub thumb_lx: i16,
    pub thumb_ly: i16,
    pub thumb_rx: i16,
    pub thumb_ry: i16,
}

impl InputReport {
    /// Extrai o estado atual de um gamepad `gilrs`
    pub fn from_gamepad(gamepad: &Gamepad, index: u8) -> Self {
        let mut report = Self {
            gamepad_index: index,
            ..Default::default()
        };

        // Mapeamento de botões XInput (bitmask)
        // D-PAD
        if gamepad.is_pressed(Button::DPadUp) {
            report.buttons |= 0x0001;
        }
        if gamepad.is_pressed(Button::DPadDown) {
            report.buttons |= 0x0002;
        }
        if gamepad.is_pressed(Button::DPadLeft) {
            report.buttons |= 0x0004;
        }
        if gamepad.is_pressed(Button::DPadRight) {
            report.buttons |= 0x0008;
        }
        // Start / Back
        if gamepad.is_pressed(Button::Start) {
            report.buttons |= 0x0010;
        }
        if gamepad.is_pressed(Button::Select) {
            report.buttons |= 0x0020;
        }
        // Thumbs
        if gamepad.is_pressed(Button::LeftThumb) {
            report.buttons |= 0x0040;
        }
        if gamepad.is_pressed(Button::RightThumb) {
            report.buttons |= 0x0080;
        }
        // Bumpers
        if gamepad.is_pressed(Button::LeftTrigger) {
            report.buttons |= 0x0100;
        } // LB
        if gamepad.is_pressed(Button::RightTrigger) {
            report.buttons |= 0x0200;
        } // RB
          // ABXY
        if gamepad.is_pressed(Button::South) {
            report.buttons |= 0x1000;
        } // A
        if gamepad.is_pressed(Button::East) {
            report.buttons |= 0x2000;
        } // B
        if gamepad.is_pressed(Button::West) {
            report.buttons |= 0x4000;
        } // X
        if gamepad.is_pressed(Button::North) {
            report.buttons |= 0x8000;
        } // Y

        // Triggers (0 a 255)
        report.left_trigger = (gamepad.value(Axis::LeftZ).max(0.0) * 255.0) as u8;
        report.right_trigger = (gamepad.value(Axis::RightZ).max(0.0) * 255.0) as u8;

        // Analógicos (-32768 a 32767)
        report.thumb_lx = (gamepad.value(Axis::LeftStickX) * 32767.0) as i16;
        report.thumb_ly = (gamepad.value(Axis::LeftStickY) * 32767.0) as i16;
        report.thumb_rx = (gamepad.value(Axis::RightStickX) * 32767.0) as i16;
        report.thumb_ry = (gamepad.value(Axis::RightStickY) * 32767.0) as i16;

        report
    }

    /// Serializa o relatório no formato binário esperado pelo xCloud via WebRTC DataChannel
    /// (Skeleton: formato precisa ser refinado conforme a especificação do xCloud)
    pub fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(32);

        // Header do protocolo de input xCloud (exemplo padrão)
        // Tipo de mensagem: Input (exemplo dummy, substituir pelo header real)
        buf.put_u8(0x01); // Message Type
        buf.put_u8(self.gamepad_index);

        // Payload XInput (Little Endian)
        buf.put_u16_le(self.buttons);
        buf.put_u8(self.left_trigger);
        buf.put_u8(self.right_trigger);
        buf.put_i16_le(self.thumb_lx);
        buf.put_i16_le(self.thumb_ly);
        buf.put_i16_le(self.thumb_rx);
        buf.put_i16_le(self.thumb_ry);

        buf.freeze()
    }
}
