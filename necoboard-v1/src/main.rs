#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    sync::atomic::{AtomicUsize, Ordering},
};

use cortex_m::{delay::Delay, interrupt::Mutex};
use defmt_rtt as _;
use embedded_hal::watchdog::{Watchdog, WatchdogEnable};
use embedded_time::duration::Extensions;
use embedded_time::rate::*;
use hal::{
    gpio::{bank0::Gpio26, FloatingInput, FunctionSpi, Pin},
    multicore::{Multicore, Stack},
    sio::Spinlock0,
    timer::Alarm,
    Adc,
};
use key_matrix::KeyMatrix;
use layout::{Layer, Layout};
use panic_probe as _;
use rp_pico::{
    entry,
    hal::{self, prelude::*, timer::CountDown, usb::UsbBus, Timer},
    pac::{self, interrupt},
};
use rustkbd_core::{
    keyboard::{DeviceInfo, Keyboard},
    split::DummyConnection,
};
use usb_device::class_prelude::UsbBusAllocator;

mod drawing;
mod filter;
mod key_matrix;
mod layout;
mod switch_identifier;

type KeyboardType = Keyboard<
    'static,
    2,
    UsbBus,
    KeyMatrix<Delay, Pin<Gpio26, FloatingInput>, 4, 4, 12>,
    DummyConnection,
    CountDown<'static>,
    Layer,
    Layout,
>;
static mut KEYBOARD: Mutex<RefCell<Option<KeyboardType>>> = Mutex::new(RefCell::new(None));
static mut ALARM: Mutex<RefCell<Option<hal::timer::Alarm0>>> = Mutex::new(RefCell::new(None));
static mut CORE1_STACK: Stack<4096> = Stack::new();

const USB_SEND_INTERVAL_MICROS: u32 = 10_000;

#[entry]
fn main() -> ! {
    // These variables must be static due to lifetime constraints
    static mut TIMER: Option<Timer> = None;
    static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

    defmt::info!("Launching necoboard v1!");

    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    // The single-cycle I/O block controls our GPIO pins
    let mut sio = hal::Sio::new(pac.SIO);
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

    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS);
    let mut alarm = timer.alarm_0().unwrap();
    alarm
        .schedule(USB_SEND_INTERVAL_MICROS.microseconds())
        .unwrap();
    alarm.enable_interrupt();
    cortex_m::interrupt::free(|cs| unsafe {
        ALARM.borrow(cs).replace(Some(alarm));
    });
    *TIMER = Some(timer);

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    *USB_BUS = Some(usb_bus);

    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];

    let mut display = drawing::display(
        pac.SPI1,
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        pins.gpio8.into_push_pull_output(),
        pins.gpio9.into_push_pull_output(),
        pins.gpio10.into_mode::<FunctionSpi>(),
        pins.gpio11.into_mode::<FunctionSpi>(),
    );

    let key_matrix = KeyMatrix::new(
        [
            pins.gpio15.into(),
            pins.gpio14.into(),
            pins.gpio13.into(),
            pins.gpio12.into(),
        ],
        [
            pins.gpio18.into(),
            pins.gpio17.into(),
            pins.gpio20.into(),
            pins.gpio19.into(),
        ],
        pins.gpio21.into(),
        pins.voltage_monitor.into(),
        pins.gpio28.into(),
        Adc::new(pac.ADC, &mut pac.RESETS),
        pins.gpio26.into_floating_input(),
        Delay::new(core.SYST, clocks.system_clock.freq().integer()),
    );

    let device_info = DeviceInfo {
        manufacturer: "necocen",
        vendor_id: 0x0c0d,
        product_id: 0x8030,
        product_name: "necoboard v1",
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
        pac::NVIC::unmask(hal::pac::Interrupt::TIMER_IRQ_0);
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

    core1
        .spawn(unsafe { &mut CORE1_STACK.mem }, move || loop {
            let state = cortex_m::interrupt::free(|cs| unsafe {
                let _lock = Spinlock0::claim();
                KEYBOARD.borrow(cs).borrow().as_ref().unwrap().get_state()
            });
            drawing::draw_state(&mut display, state);
            display.flush().ok();
        })
        .unwrap();

    watchdog.pause_on_debug(true);
    watchdog.start(1_000_000.microseconds());

    loop {
        cortex_m::interrupt::free(|cs| unsafe {
            let _lock = Spinlock0::claim();
            KEYBOARD
                .borrow(cs)
                .borrow()
                .as_ref()
                .map(Keyboard::main_loop);
        });
        watchdog.feed();
    }
}

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    cortex_m::interrupt::free(|cs| unsafe {
        let _lock = Spinlock0::claim();
        KEYBOARD
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(Keyboard::usb_poll);
    });
}

#[allow(non_snake_case)]
#[interrupt]
fn TIMER_IRQ_0() {
    cortex_m::interrupt::free(|cs| unsafe {
        let mut alarm = ALARM.borrow(cs).borrow_mut();
        let alarm = alarm.as_mut().unwrap();
        alarm.clear_interrupt();
        alarm
            .schedule(USB_SEND_INTERVAL_MICROS.microseconds())
            .unwrap();
        let _lock = Spinlock0::claim();
        alarm.enable_interrupt();
        if let Some(Err(e)) = KEYBOARD
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(Keyboard::send_keys)
        {
            defmt::warn!("UsbError: {}", defmt::Debug2Format(&e));
        }
    });
}
