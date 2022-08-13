use defmt::Format;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
#[repr(u16)]
#[allow(non_camel_case_types, dead_code)]
pub enum Key {
    // FIXME: We need shorter notation.
    None = 0x0000,
    Transparent,
    A = 0x0004,
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
    LeftControl = 0x00e0,
    LeftShift,
    LeftAlt,
    LeftGui, // Win key(Windows), Command key(Mac), Meta key
    RightControl,
    RightShift,
    RightAlt,
    RightGui, // Win key(Windows), Command key(Mac), Meta key
    MediaZero = 0x1000,
    MediaPlay = 0x10B0,
    MediaPause = 0x10B1,
    MediaRecord = 0x10B2,
    MediaNextTrack = 0x10B5,
    MediaPrevTrack = 0x10B6,
    MediaStop = 0x10B7,
    MediaRandomPlay = 0x10B9,
    MediaRepeat = 0x10BC,
    MediaPlayPause = 0x10CD,
    MediaMute = 0x10E2,
    MediaVolumeIncrement = 0x10E9,
    MediaVolumeDecrement = 0x10EA,
    Tilde = 0xe135,
    Exclamation = 0xe11e,
    At = 0xe11f,
    Hash = 0xe120,
    Dollar = 0xe121,
    Percent = 0xe122,
    Circumflex = 0xe123,
    Ampersand = 0xe124,
    Asterisk = 0xe125,
    LeftParenthesis = 0xe126,
    RightParenthesis = 0xe127,
    LowLine = 0xe12d,
    Plus = 0xe12e,
    LeftCurlyBracket = 0xe12f,
    RightCurlyBracket = 0xe130,
    VerticalBar = 0xe131,
    Colon = 0xe133,
    Quotation = 0xe134,
    LessThan = 0xe136,
    GreaterThan = 0xe137,
    Question = 0xe138,
}

impl Key {
    pub fn is_noop(&self) -> bool {
        *self as u16 <= 0x0001
    }

    pub fn is_modifier_key(&self) -> bool {
        (*self as u16) >= 0x00e0 && (*self as u16) <= 0x00e7
    }

    pub fn is_modified_key(&self) -> bool {
        (*self as u16 >> 8) >= 0x00e0
            && (*self as u16 >> 8) <= 0x00e7
            && (*self as u16 & 0xff) >= 0x0004
            && (*self as u16 & 0xff) < 0x00e0
    }

    pub fn is_keyboard_key(&self) -> bool {
        *self as u16 >= 0x0004 && (*self as u16) < 0x00e0
    }

    pub fn key_code(&self) -> Option<u8> {
        if self.is_modified_key() || self.is_keyboard_key() {
            Some((*self as u16 & 0xff) as u8)
        } else {
            None
        }
    }

    pub fn is_media_key(&self) -> bool {
        (*self as u16) >= 0x1000 && (*self as u16) < 0x2000
    }

    pub(crate) fn modifier_key_flag(&self) -> u8 {
        if self.is_modifier_key() {
            1 << ((*self as u16) - 0x00e0)
        } else if self.is_modified_key() {
            1 << ((*self as u16 >> 8) - 0x00e0)
        } else {
            0x00
        }
    }

    pub(crate) fn media_usage_id(&self) -> u16 {
        if self.is_media_key() {
            (*self as u16) & 0x0fff
        } else {
            0x0000
        }
    }
}

impl From<Key> for char {
    fn from(key: Key) -> Self {
        static CHARS: &[u8] = (r##"abcdefghijklmnopqrstuvwxyz1234567890REBT -=[]\#;'`,./ FFFFFFFFFFFF              /*-+R1234567890.\  =FFFFFFFFFFFF                 ,=IIIIIIIIILLLLLLLLLB    E      "##).as_bytes();
        match key as u8 {
            0x0004..=0x00a4 => CHARS[(key as usize) - 0x0004] as char,
            _ => ' ',
        }
    }
}
