use usbd_hid::descriptor::generator_prelude::*;
use usbd_hid_macros::gen_hid_descriptor;

/**
 * cf. https://hikalium.hatenablog.jp/entry/2021/12/31/150738
 */
#[gen_hid_descriptor(
    (collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = KEYBOARD) = {
        (usage_page = KEYBOARD, usage_min = 0xe0, usage_max = 0xe7) = {
            #[packed_bits 8] #[item_settings data,variable,absolute] modifier=input;
        };
        (usage_min = 0x00, usage_max = 0xff) = {
            #[item_settings constant,variable,absolute] reserved=input;
        };
        (usage_page = KEYBOARD, usage_min = 0x00, usage_max = 0xdd) = {
            #[item_settings data,array,absolute] key_codes=input;
        };
    }
)]
#[repr(C)]
pub struct HidKeyboardReport {
    pub modifier: u8,
    pub reserved: u8,
    pub key_codes: [u8; 6],
}
