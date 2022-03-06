#![no_std]
#![no_main]
extern crate panic_halt;

use core::cell::RefCell;

use cortex_m::{
    delay::{self, Delay},
    interrupt::Mutex,
};
use cortex_m_rt::entry;
use embedded_time::rate::*;
use key_matrix::KeyMatrix;
use keyboard_core::keyboard::Keyboard;
use rp_pico::{
    hal::{
        self,
        gpio::{
            bank0::{Gpio6, Gpio7},
            Function, Pin, I2C as GpioI2C,
        },
        prelude::*,
        I2C,
    },
    pac::{self, interrupt, I2C1},
};
use ssd1306::{
    mode::DisplayConfig, prelude::I2CInterface, rotation::DisplayRotation, size::DisplaySize128x32,
    I2CDisplayInterface, Ssd1306,
};
use ssd1306_display::Ssd1306Display;
use usb_device::{
    class_prelude::UsbBusAllocator,
    device::{UsbDevice, UsbDeviceBuilder, UsbDeviceState, UsbVidPid},
};
use usbd_hid::{descriptor::generator_prelude::*, hid_class::HIDClass};
use usbd_hid_macros::gen_hid_descriptor;

mod key_matrix;
mod ssd1306_display;

/// The USB Bus Driver (shared with the interrupt).
static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;
/// The USB Human Interface Device Driver (shared with the interrupt).
static mut USB_HID: Option<HIDClass<hal::usb::UsbBus>> = None;
/// The USB Device Driver (shared with the interrupt).
static mut USB_DEVICE: Option<UsbDevice<hal::usb::UsbBus>> = None;

type KeyboardType = Keyboard<
    KeyMatrix<Delay, 2, 2>,
    Ssd1306Display<
        I2CInterface<I2C<I2C1, (Pin<Gpio6, Function<GpioI2C>>, Pin<Gpio7, Function<GpioI2C>>)>>,
        DisplaySize128x32,
    >,
>;
static KEYBOARD: Mutex<RefCell<Option<KeyboardType>>> = Mutex::new(RefCell::new(None));

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
    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
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
    let mut delay = delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

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

    let i2c = I2C::i2c1(
        pac.I2C1,
        pins.gpio6.into_mode(),
        pins.gpio7.into_mode(),
        400.kHz(),
        &mut pac.RESETS,
        clocks.system_clock,
    );
    let interface = I2CDisplayInterface::new(i2c);
    let mut ssd1306 = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    ssd1306.init().ok();
    let display = Ssd1306Display(ssd1306);

    // controller-receiver detection
    delay.delay_ms(500);
    let is_recv = cortex_m::interrupt::free(|_| unsafe {
        USB_DEVICE.as_ref().unwrap().state() == UsbDeviceState::Default
    });

    let key_matrix = KeyMatrix::new(
        [pins.gpio16.into(), pins.gpio17.into()],
        [pins.gpio14.into(), pins.gpio15.into()],
        delay,
    );
    let keyboard = Keyboard::new(key_matrix, display, is_recv);
    cortex_m::interrupt::free(|cs| {
        KEYBOARD.borrow(cs).replace(Some(keyboard));
    });

    loop {
        cortex_m::interrupt::free(|cs| unsafe {
            let key_codes = KEYBOARD.borrow(cs).borrow().as_ref().unwrap().main_loop();
            if !is_recv {
                let report = KeyboardReport {
                    modifier: 0,
                    reserved: 0,
                    key_codes,
                };
                USB_HID.as_mut().map(|hid| hid.push_input(&report));
            }
        });
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
