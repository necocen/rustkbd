#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    sync::atomic::{AtomicUsize, Ordering},
};
use cortex_m::{delay::Delay, interrupt::Mutex};
use defmt_rtt as _;
use fugit::{ExtU32, MicrosDurationU32};
use hal::{entry, Clock as _, Timer};
use key_matrix::KeyMatrix;
use layout::Layout;
use panic_probe as _;
use rp2040_hal::{
    self as hal,
    adc::AdcPin,
    gpio,
    multicore::{Multicore, Stack},
    pac::{self, interrupt},
    sio::Spinlock0,
    timer::Alarm,
    usb::UsbBus,
    Adc, Sio, Watchdog,
};
use rustkbd::{
    keyboard::Controller,
    usb::{DeviceInfo, UsbCommunicator},
};
use usb_device::class_prelude::UsbBusAllocator;

mod buffer;
mod drawing;
mod kalman_filter;
mod key_matrix;
mod layout;
mod switch_identifier;

type KeyboardType = Controller<
    2,
    12,
    UsbCommunicator<'static, UsbBus>,
    KeyMatrix<
        Delay,
        AdcPin<gpio::Pin<gpio::bank0::Gpio26, gpio::FunctionNull, gpio::PullDown>>,
        4,
        4,
        12,
    >,
    Layout,
>;
static mut KEYBOARD: Mutex<RefCell<Option<KeyboardType>>> = Mutex::new(RefCell::new(None));
static mut ALARM: Mutex<RefCell<Option<hal::timer::Alarm0>>> = Mutex::new(RefCell::new(None));
static mut CORE1_STACK: Stack<4096> = Stack::new();

const USB_SEND_INTERVAL: MicrosDurationU32 = MicrosDurationU32::millis(10);

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
/// Note: This boot block is not necessary when using a rp-hal based BSP
/// as the BSPs already perform this step.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

#[entry]
fn main() -> ! {
    // These variables must be static due to lifetime constraints
    static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

    defmt::info!("Launching necoboard v1!");

    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    // The single-cycle I/O block controls our GPIO pins
    let mut sio = Sio::new(pac.SIO);
    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        12_000_000,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let mut alarm = timer.alarm_0().unwrap();
    alarm.schedule(USB_SEND_INTERVAL).unwrap();
    alarm.enable_interrupt();
    cortex_m::interrupt::free(|cs| unsafe {
        ALARM.borrow(cs).replace(Some(alarm));
    });

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
        pins.gpio8.into_function(),
        pins.gpio9.into_function(),
        pins.gpio10.into_function(),
        pins.gpio11.into_function(),
    );

    let key_matrix = KeyMatrix::new(
        [
            pins.gpio15.reconfigure().into_dyn_pin(),
            pins.gpio14.reconfigure().into_dyn_pin(),
            pins.gpio13.reconfigure().into_dyn_pin(),
            pins.gpio12.reconfigure().into_dyn_pin(),
        ],
        [
            pins.gpio18.reconfigure().into_dyn_pin(),
            pins.gpio17.reconfigure().into_dyn_pin(),
            pins.gpio20.reconfigure().into_dyn_pin(),
            pins.gpio19.reconfigure().into_dyn_pin(),
        ],
        pins.gpio21.reconfigure().into_dyn_pin(),
        pins.gpio29.reconfigure().into_dyn_pin(),
        pins.gpio28.reconfigure().into_dyn_pin(),
        Adc::new(pac.ADC, &mut pac.RESETS),
        AdcPin::new(pins.gpio26).unwrap(),
        Delay::new(core.SYST, clocks.system_clock.freq().to_Hz()),
    );

    let device_info = DeviceInfo {
        manufacturer: "necocen",
        vendor_id: 0x0c0d,
        product_id: 0x8030,
        product_name: "necoboard v1",
        serial_number: "17",
    };

    let keyboard = Controller::new(
        UsbCommunicator::new(device_info, USB_BUS.as_ref().unwrap()),
        key_matrix,
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
    watchdog.start(1.secs());

    loop {
        cortex_m::interrupt::free(|cs| unsafe {
            let _lock = Spinlock0::claim();
            KEYBOARD
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .map(Controller::main_loop);
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
            .borrow_mut()
            .as_mut()
            .map(|keyboard| keyboard.communicator.poll())
    });
}

#[allow(non_snake_case)]
#[interrupt]
fn TIMER_IRQ_0() {
    cortex_m::interrupt::free(|cs| unsafe {
        let _lock = Spinlock0::claim();
        let mut alarm = ALARM.borrow(cs).borrow_mut();
        let alarm = alarm.as_mut().unwrap();
        alarm.clear_interrupt();
        alarm.schedule(USB_SEND_INTERVAL).unwrap();
        alarm.enable_interrupt();
        if let Some(Err(e)) = KEYBOARD
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(Controller::send_keys)
        {
            defmt::warn!("UsbError: {}", defmt::Debug2Format(&e));
        }
    });
}
