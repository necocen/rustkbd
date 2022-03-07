#![no_std]
#![no_main]
extern crate panic_halt;

use core::cell::RefCell;

use cortex_m::{
    delay::{self, Delay},
    interrupt::Mutex,
};
use cortex_m_rt::entry;
use embedded_hal::digital::v2::InputPin;
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
        usb::UsbBus,
        I2C,
    },
    pac::{self, interrupt, I2C1},
};
use ssd1306::{
    mode::DisplayConfig, prelude::I2CInterface, rotation::DisplayRotation, size::DisplaySize128x32,
    I2CDisplayInterface, Ssd1306,
};
use ssd1306_display::Ssd1306Display;
use usb_device::class_prelude::UsbBusAllocator;

mod key_matrix;
mod ssd1306_display;

/// The USB Bus Driver (shared with the interrupt).
static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

type KeyboardType = Keyboard<
    'static,
    UsbBus,
    KeyMatrix<Delay, 2, 2>,
    Ssd1306Display<
        I2CInterface<I2C<I2C1, (Pin<Gpio6, Function<GpioI2C>>, Pin<Gpio7, Function<GpioI2C>>)>>,
        DisplaySize128x32,
    >,
>;
static KEYBOARD: Mutex<RefCell<Option<KeyboardType>>> = Mutex::new(RefCell::new(None));

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
    let delay = delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

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

    let key_matrix = KeyMatrix::new(
        [pins.gpio16.into(), pins.gpio17.into()],
        [pins.gpio14.into(), pins.gpio15.into()],
        delay,
    );
    let is_left_hand = pins.gpio22.into_pull_up_input().is_low().unwrap();
    let keyboard = Keyboard::new(
        unsafe { USB_BUS.as_ref().unwrap() },
        key_matrix,
        display,
        is_left_hand,
    );
    cortex_m::interrupt::free(|cs| {
        KEYBOARD.borrow(cs).replace(Some(keyboard));
    });
    loop {
        cortex_m::interrupt::free(|cs| {
            KEYBOARD.borrow(cs).borrow().as_ref().unwrap().main_loop();
        });
    }
}

#[allow(non_snake_case)]
#[interrupt]
unsafe fn USBCTRL_IRQ() {
    cortex_m::interrupt::free(|cs| {
        KEYBOARD.borrow(cs).borrow().as_ref().unwrap().usb_poll();
    });
}
