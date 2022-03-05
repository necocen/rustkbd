#![no_std]
#![no_main]
extern crate panic_halt;

use cortex_m::delay;
use cortex_m_rt::entry;
use keyboard_core::keyboard::Keyboard;
use rp_pico::{
    hal::{self, prelude::*},
    pac::{self, interrupt},
};
// Time handling traits
use embedded_time::rate::*;
// GPIO traits
use embedded_hal::digital::v2::OutputPin;
use usb_device::{
    class_prelude::UsbBusAllocator,
    device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
};
use usbd_hid::{descriptor::generator_prelude::*, hid_class::HIDClass};
use usbd_hid_macros::gen_hid_descriptor;

//mod key_matrix;

/// The USB Bus Driver (shared with the interrupt).
static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;
/// The USB Human Interface Device Driver (shared with the interrupt).
static mut USB_HID: Option<HIDClass<hal::usb::UsbBus>> = None;
/// The USB Device Driver (shared with the interrupt).
static mut USB_DEVICE: Option<UsbDevice<hal::usb::UsbBus>> = None;

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
struct KeyboardReport {
    pub modifier: u8,
    pub reserved: u8,
    pub key_codes: [u8; 6],
}

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    // Configure the clocks
    //
    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    unsafe {
        // Note (safety): This is safe as interrupts haven't been started yet
        USB_BUS = Some(usb_bus);
    }

    let bus_ref = unsafe { USB_BUS.as_ref().unwrap() };
    let usb_hid = HIDClass::new(bus_ref, KeyboardReport::desc(), 10);
    unsafe {
        USB_HID = Some(usb_hid);
    }

    let usb_device = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0xfeed, 0x802f))
        .manufacturer("necocen")
        .product("necoboard")
        .serial_number("17")
        .device_class(0) // HID Device?
        .build();
    unsafe {
        USB_DEVICE = Some(usb_device);
    }

    unsafe {
        // Enable the USB interrupt
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
    };

    let mut delay = delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    // The single-cycle I/O block controls our GPIO pins
    //let sio = hal::Sio::new(pac.SIO);

    // let pins = rp_pico::Pins::new(
    //     pac.IO_BANK0,
    //     pac.PADS_BANK0,
    //     sio.gpio_bank0,
    //     &mut pac.RESETS,
    // );

    loop {
        cortex_m::interrupt::free(|_| unsafe {
            let report = KeyboardReport {
                modifier: 0,
                reserved: 0,
                key_codes: [0x1e, 0, 0, 0, 0, 0],
            };
            USB_HID.as_mut().map(|hid| hid.push_input(&report));
        });
        delay.delay_ms(10);
    }
}

#[allow(non_snake_case)]
#[interrupt]
unsafe fn USBCTRL_IRQ() {
    // Handle USB request
    let usb_dev = USB_DEVICE.as_mut().unwrap();
    let usb_hid = USB_HID.as_mut().unwrap();
    usb_dev.poll(&mut [usb_hid]);
}
