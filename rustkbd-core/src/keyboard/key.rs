#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(non_camel_case_types, dead_code)]
pub enum Key {
    // FIXME: We need shorter notation.
    A = 0x04,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Digit1_Exclamation,
    Digit2_At,
    Digit3_Number,
    Digit4_Dollar,
    Digit5_Percent,
    Digit6_Circumflex,
    Digit7_Ampersand,
    Digit8_Asterisk,
    Digit9_LeftParenthesis,
    Digit0_RightParenthesis,
    Enter,
    Escape,
    Delete,
    Tab,
    Space,
    HyphenMinus_LowLine,
    Equal_Plus,
    LeftSquareBracket_LeftCurlyBracket,
    RightSquareBracket_RightCurlyBracket,
    Backslash_VerticalBar,
    NonUs_Number_Tilde,
    Semicolon_Colon,
    Apostrophe_Quotation,
    Grave_Tilde,
    Comma_LessThan,
    Period_GreaterThan,
    Slash_Question,
    CapsLock,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    PrintScreen,
    ScrollLock,
    Pause,
    Insert,
    Home,
    PageUp,
    DeleteForward,
    End,
    PageDown,
    RightArrow,
    LeftArrow,
    DownArrow,
    UpArrow,
    Keypad_NumLock_Clear,
    Keypad_Slash,
    Keypad_Asterisk,
    Keypad_HyphenMinus,
    Keypad_Plus,
    Keypad_Enter,
    Keypad_Digit1_End,
    Keypad_Digit2_DownArrow,
    Keypad_Digit3_PageDown,
    Keypad_Digit4_LeftArrow,
    Keypad_Digit5,
    Keypad_Digit6_RightArrow,
    Keypad_Digit7_Home,
    Keypad_Digit8_UpArrow,
    Keypad_Digit9_PageUp,
    Keypad_Digit0_Insert,
    Keypad_Period_Delete,
    NonUs_BackSlash_VerticalBar,
    Application,
    Power,
    Keypad_Equal,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Execute,
    Help,
    Menu,
    Select,
    Stop,
    Again,
    Undo,
    Cut,
    Copy,
    Paste,
    Find,
    Mute,
    VolumeUp,
    VolumeDown,
    LockingCapsLock,
    LockingNumLock,
    LockingScrollLock,
    Keypad_Comma,
    Keypad_EqualSign, // Used on AS/400 keyboards
    International1,   // LowLine, VerticalBar, Backslash, ろ
    International2,   // カタカナ／ひらがな, かな(PC98)
    International3,   // LowLine, LeftSquareBracket, ￥, HyphenMinus
    International4,   // 前候補, 変換, XFER(PC98)
    International5,   // 無変換, NFER(PC98)
    International6,   // Comma(PC98)
    International7,   // Toggle Double-Byte/Single-Byte mode
    International8,   // Undefined
    International9,   // Undefined
    Lang1,            // Hangul/English toggle key
    Lang2,            // Hanja conversion key
    Lang3,            // Katakana key
    Lang4,            // Hiragana key
    Lang5,            // Zenkaku/Hankaku key
    Lang6,            // Reserved
    Lang7,            // Reserved
    Lang8,            // Reserved
    Lang9,            // Reserved
    AlternateErase,   // e.g. Erase-Eaze key
    SysReq_Attention,
    Cancel,
    Clear,
    Prior,
    Return,
    Separator,
    Out,
    Oper,
    Clear_Again,
    CrSel_Props,
    ExSel,
    LeftControl = 0xe0,
    LeftShift,
    LeftAlt,
    LeftGui, // Win key(Windows), Command key(Mac), Meta key
    RightControl,
    RightShift,
    RightAlt,
    RightGui, // Win key(Windows), Command key(Mac), Meta key
}

impl Key {
    pub fn is_modifier_key(&self) -> bool {
        (*self as u8) >= 0xe0
    }

    pub(crate) fn modifier_key_flag(&self) -> u8 {
        if self.is_modifier_key() {
            1 << ((*self as u8) - 0xe0)
        } else {
            0x00
        }
    }
}

impl From<Key> for char {
    fn from(key: Key) -> Self {
        static CHARS: &[u8] = (r##"abcdefghijklmnopqrstuvwxyz1234567890REBT -=[]\#;'`,./ FFFFFFFFFFFF              /*-+R1234567890.\  =FFFFFFFFFFFF                 ,=IIIIIIIIILLLLLLLLLB    E      "##).as_bytes();
        match key as u8 {
            0x04..=0xa4 => CHARS[(key as usize) - 0x04] as char,
            _ => ' ',
        }
    }
}
