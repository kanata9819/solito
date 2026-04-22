use winit::keyboard::KeyCode;

pub enum ParseError {
    InvalidCode(KeyCode),
}

pub enum CodeKind {
    Char(char),
    Special,
    Function,
}

pub enum ParseResult {
    Ok(CodeKind),
    Err(ParseError),
}

pub fn parse(code: &KeyCode, is_pressed: bool) -> ParseResult {
    match (code, is_pressed) {
        (KeyCode::KeyA, true) => ParseResult::Ok(CodeKind::Char('A')),
        (KeyCode::KeyB, true) => ParseResult::Ok(CodeKind::Char('B')),
        (KeyCode::KeyC, true) => ParseResult::Ok(CodeKind::Char('C')),
        (KeyCode::KeyD, true) => ParseResult::Ok(CodeKind::Char('D')),
        (KeyCode::KeyE, true) => ParseResult::Ok(CodeKind::Char('E')),
        (KeyCode::KeyF, true) => ParseResult::Ok(CodeKind::Char('F')),
        (KeyCode::KeyG, true) => ParseResult::Ok(CodeKind::Char('G')),
        (KeyCode::KeyH, true) => ParseResult::Ok(CodeKind::Char('H')),
        (KeyCode::KeyI, true) => ParseResult::Ok(CodeKind::Char('I')),
        (KeyCode::KeyJ, true) => ParseResult::Ok(CodeKind::Char('J')),
        (KeyCode::KeyK, true) => ParseResult::Ok(CodeKind::Char('K')),
        (KeyCode::KeyL, true) => ParseResult::Ok(CodeKind::Char('L')),
        (KeyCode::KeyM, true) => ParseResult::Ok(CodeKind::Char('M')),
        (KeyCode::KeyN, true) => ParseResult::Ok(CodeKind::Char('N')),
        (KeyCode::KeyO, true) => ParseResult::Ok(CodeKind::Char('O')),
        (KeyCode::KeyP, true) => ParseResult::Ok(CodeKind::Char('P')),
        (KeyCode::KeyQ, true) => ParseResult::Ok(CodeKind::Char('Q')),
        (KeyCode::KeyR, true) => ParseResult::Ok(CodeKind::Char('R')),
        (KeyCode::KeyS, true) => ParseResult::Ok(CodeKind::Char('S')),
        (KeyCode::KeyT, true) => ParseResult::Ok(CodeKind::Char('T')),
        (KeyCode::KeyU, true) => ParseResult::Ok(CodeKind::Char('U')),
        (KeyCode::KeyV, true) => ParseResult::Ok(CodeKind::Char('V')),
        (KeyCode::KeyW, true) => ParseResult::Ok(CodeKind::Char('W')),
        (KeyCode::KeyX, true) => ParseResult::Ok(CodeKind::Char('X')),
        (KeyCode::KeyY, true) => ParseResult::Ok(CodeKind::Char('Y')),
        (KeyCode::KeyZ, true) => ParseResult::Ok(CodeKind::Char('Z')),
        (KeyCode::Digit0, true) => ParseResult::Ok(CodeKind::Char('0')),
        (KeyCode::Digit1, true) => ParseResult::Ok(CodeKind::Char('1')),
        (KeyCode::Digit2, true) => ParseResult::Ok(CodeKind::Char('2')),
        (KeyCode::Digit3, true) => ParseResult::Ok(CodeKind::Char('3')),
        (KeyCode::Digit4, true) => ParseResult::Ok(CodeKind::Char('4')),
        (KeyCode::Digit5, true) => ParseResult::Ok(CodeKind::Char('5')),
        (KeyCode::Digit6, true) => ParseResult::Ok(CodeKind::Char('6')),
        (KeyCode::Digit7, true) => ParseResult::Ok(CodeKind::Char('7')),
        (KeyCode::Digit8, true) => ParseResult::Ok(CodeKind::Char('8')),
        (KeyCode::Digit9, true) => ParseResult::Ok(CodeKind::Char('9')),
        (KeyCode::Space, true) => ParseResult::Ok(CodeKind::Char(' ')),
        (KeyCode::Comma, true) => ParseResult::Ok(CodeKind::Char(',')),
        (KeyCode::Period, true) => ParseResult::Ok(CodeKind::Char('.')),
        (KeyCode::NumpadAdd, true) => ParseResult::Ok(CodeKind::Char('+')),
        (KeyCode::NumpadSubtract, true) => ParseResult::Ok(CodeKind::Char('-')),
        (KeyCode::NumpadMultiply, true) => ParseResult::Ok(CodeKind::Char('*')),
        (KeyCode::NumpadDivide, true) => ParseResult::Ok(CodeKind::Char('/')),
        (KeyCode::NumpadEnter, true) => ParseResult::Ok(CodeKind::Char('\n')),
        (KeyCode::NumpadDecimal, true) => ParseResult::Ok(CodeKind::Char('.')),
        (KeyCode::Backspace, true) => ParseResult::Ok(CodeKind::Char('\u{8}')),
        (KeyCode::Escape, true) => ParseResult::Ok(CodeKind::Char('\u{1B}')),
        (KeyCode::Tab, true) => ParseResult::Ok(CodeKind::Char('\t')),
        (KeyCode::Enter, true) => ParseResult::Ok(CodeKind::Char('\n')),
        (KeyCode::Delete, true) => ParseResult::Ok(CodeKind::Char('\u{7F}')),
        (KeyCode::Insert, true) => ParseResult::Ok(CodeKind::Char('\u{0C}')),
        (KeyCode::ShiftLeft, true)
        | (KeyCode::ShiftRight, true)
        | (KeyCode::Hiragana, true)
        | (KeyCode::KanaMode, true)
        | (KeyCode::Katakana, true)
        | (KeyCode::CapsLock, true) => ParseResult::Ok(CodeKind::Special),
        (KeyCode::F1, true)
        | (KeyCode::F2, true)
        | (KeyCode::F3, true)
        | (KeyCode::F4, true)
        | (KeyCode::F5, true)
        | (KeyCode::F6, true)
        | (KeyCode::F7, true)
        | (KeyCode::F8, true)
        | (KeyCode::F9, true) => ParseResult::Ok(CodeKind::Function),
        _ => ParseResult::Err(ParseError::InvalidCode(*code)),
    }
}
