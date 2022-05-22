#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    sync::atomic::{AtomicUsize, Ordering},
};

use cortex_m::{
    delay::{self, Delay},
    interrupt::Mutex,
};
use cortex_m_rt::entry;
use defmt_rtt as _;
use embedded_time::rate::*;
use hal::{
    gpio::{bank0::Gpio26, FloatingInput, Pin},
    Adc,
};
use key_matrix::KeyMatrix;
use layout::{Layer, Layout};
use panic_probe as _;
use rp_pico::{
    hal::{self, prelude::*, timer::CountDown, usb::UsbBus, Timer},
    pac::{self, interrupt},
};
use rustkbd_core::{
    keyboard::{DeviceInfo, Keyboard},
    split::DummyConnection,
};
use usb_device::class_prelude::UsbBusAllocator;

mod key_matrix;
mod layout;
mod switch_identifier;

type KeyboardType = Keyboard<
    'static,
    2,
    UsbBus,
    KeyMatrix<Delay, Pin<Gpio26, FloatingInput>, 2, 3, 2>,
    DummyConnection,
    CountDown<'static>,
    Layer,
    Layout,
>;
static mut KEYBOARD: Mutex<RefCell<Option<KeyboardType>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    // These variables must be static due to lifetime constraints
    static mut TIMER: Option<Timer> = None;
    static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

    defmt::info!("Launching necoboard-petit EC!");

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
    *TIMER = Some(Timer::new(pac.TIMER, &mut pac.RESETS));
    let adc = Adc::new(pac.ADC, &mut pac.RESETS);

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    *USB_BUS = Some(usb_bus);

    delay.delay_ms(100);

    let key_matrix = KeyMatrix::new(
        [pins.gpio14.into(), pins.gpio15.into()],
        [pins.gpio16.into(), pins.gpio17.into(), pins.gpio18.into()],
        pins.gpio19.into(),
        pins.gpio21.into(),
        pins.gpio20.into(),
        adc,
        pins.gpio26.into_floating_input(),
        delay,
    );

    let device_info = DeviceInfo {
        manufacturer: "necocen",
        vendor_id: 0x0c0d,
        product_id: 0x8030,
        product_name: "necoboard petit EC",
        serial_number: "17",
    };

    let keyboard = Keyboard::new(
        USB_BUS.as_ref().unwrap(),
        device_info,
        key_matrix,
        TIMER.as_ref().unwrap().count_down(),
        Layout::default(),
    );
    cortex_m::interrupt::free(|cs| unsafe {
        KEYBOARD.borrow(cs).replace(Some(keyboard));
    });

    unsafe {
        // Enable the USB interrupt
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
        //pac::NVIC::unmask(hal::pac::Interrupt::UART0_IRQ);
    }
    // defmt のタイムスタンプを実装します
    // タイマを使えば、起動からの時間を表示したりできます
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    defmt::timestamp!("{=usize}", {
        // NOTE(no-CAS) `timestamps` runs with interrupts disabled
        let n = COUNT.load(Ordering::Relaxed);
        COUNT.store(n + 1, Ordering::Relaxed);
        n
    });

    loop {
        cortex_m::interrupt::free(|cs| unsafe {
            let keyboard = KEYBOARD.borrow(cs).borrow();
            let keyboard = keyboard.as_ref().unwrap();
            keyboard.main_loop();
        });
    }
}

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    cortex_m::interrupt::free(|cs| unsafe {
        KEYBOARD
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(Keyboard::usb_poll)
    });
}
