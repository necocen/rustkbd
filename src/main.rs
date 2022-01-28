#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
extern crate panic_halt;

use core::{borrow::Borrow, cell::RefCell, ops::DerefMut};

use atmega_hal::{
    clock::MHz16,
    delay::Delay,
    pac, pins,
    port::{mode::Output, Pin, PB6},
    Peripherals, Pins,
};
use avr_device::interrupt::{self, Mutex};
use embedded_hal::blocking::delay::DelayMs;

static MY_USB: Mutex<RefCell<Option<pac::USB_DEVICE>>> = Mutex::new(RefCell::new(None));
static MY_PLL: Mutex<RefCell<Option<pac::PLL>>> = Mutex::new(RefCell::new(None));
static MY_B6: Mutex<RefCell<Option<Pin<Output, PB6>>>> = Mutex::new(RefCell::new(None));

fn configure_endpoint(dev: &atmega_hal::pac::USB_DEVICE, addr: u8) -> u8 {
    let addr_tmp = addr & 0x0F;
    for i in addr_tmp..0x07 {
        let mut uecfg0x_tmp = 0u8;
        let mut uecfg1x_tmp = 0u8;
        let mut ueienx_tmp = 0u8;
        unsafe { dev.uenum.write(|w| w.bits(addr)) };
        if i == addr_tmp {
            uecfg0x_tmp = 0 << 6; // Type << EPTYPE0
            if (addr & 0x80) != 0 {
                // ENDPOINT_DIR_MASK_IN
                uecfg0x_tmp |= 0x01 << 0 // EPDIR
            }
            // if doubleBank {
            //      uecfg1x_tmp |= 0x01 << 2 // EPBK0
            // }
            let mut tmp = 0x08u8;
            let mut epsize = 0x00u8;
            while tmp < 0x08 {
                // ENDPOINT_CONTROL_SIZE
                epsize += 1;
                tmp <<= 1;
            }
            uecfg1x_tmp |= epsize << 4; // EPSIZE0
            uecfg1x_tmp |= 0x01 << 1; // ALLOC
            ueienx_tmp = 0x00;
        } else {
            uecfg0x_tmp = dev.uecfg0x.read().bits();
            uecfg1x_tmp = dev.uecfg1x.read().bits();
            ueienx_tmp = dev.ueienx.read().bits();
        }

        if (uecfg1x_tmp & (0x01 << 1)) == 0 {
            // ALLOC
            continue;
        }
        dev.ueconx.modify(|_, w| w.epen().clear_bit());
        dev.uecfg1x.modify(|_, w| w.alloc().clear_bit());
        dev.ueconx.modify(|_, w| w.epen().set_bit());
        unsafe { dev.uecfg0x.write(|w| w.bits(uecfg0x_tmp)) };
        unsafe { dev.uecfg1x.write(|w| w.bits(uecfg1x_tmp)) };
        unsafe { dev.ueienx.write(|w| w.bits(ueienx_tmp)) };
        if dev.uesta0x.read().cfgok().bit_is_set() {
            return 0;
        }
    }

    return 1;
}

#[atmega_hal::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();
    let dev = dp.USB_DEVICE;
    let pll = dp.PLL;
    let pins = pins!(dp);
    let mut delay = Delay::<MHz16>::new();

    interrupt::free(|cs| {
        MY_USB.borrow(cs).replace(Some(dev));
        MY_PLL.borrow(cs).replace(Some(pll));
        MY_B6.borrow(cs).replace(Some(pins.pb6.into_output()));
    });
    unsafe {
        interrupt::enable();
    }

    usb_init();

    loop {
        //interrupt::free(|cs| MY_B6.borrow(cs).borrow_mut().as_mut().unwrap().toggle());
        // 0.1秒待機
        delay.delay_ms(1000u16);
    }
}

fn usb_init() {
    interrupt::free(|cs| {
        if let (Some(dev), Some(pll)) = (
            MY_USB.borrow(cs).borrow().as_ref(),
            MY_PLL.borrow(cs).borrow().as_ref(),
        ) {
            dev.usbcon.reset();
            dev.uhwcon.reset();
            dev.udcon.reset();
            dev.udien.reset();
            // 一瞬切った方がいいらしい cf. https://kampi.gitbook.io/avr/lets-use-usb/initialize-the-usb-as-a-usb-device
            dev.usbcon.modify(|_, w| w.usbe().clear_bit());
            dev.usbcon.write(|w| w.usbe().set_bit());
            // After a reset, the USB device mode is automatically activated because the external UID pin of the microcontroller is deactivated (UIDE = 0) and the UIMOD bit is set. Furthermore, the PLL for the USB module gets turned off during a reset by setting the FRZCLK bit in the USBCON register. This bit must also be deleted after a reset.
            dev.usbcon.write(|w| w.frzclk().clear_bit());
            // It is recommended to use the internal voltage regulator to ensure the supply voltage of the data lines.
            dev.uhwcon.modify(|_, w| w.uvrege().set_bit());
            // In order for the microcontroller to attach to the USB and the host to recognize the new device, the VBUS pad must also be activated via the OTGPADE bit in the USBCON register.
            dev.usbcon.write(|w| w.otgpade().set_bit().usbe().set_bit());
            dev.usbcon.modify(|_, w| w.usbe().set_bit());
            // low speed mode (full speed modeのときは外部発振子が必要らしい？)
            dev.udcon.modify(|_, w| w.lsm().set_bit());
            // Only the VBUS interrupt is currently required, as this is used for plug-in detection. This interrupt is activated via the VBUSTE bit in the USBCON register.
            dev.usbcon.modify(|_, w| w.vbuste().set_bit());
            // ?
            dev.udien
                .modify(|_, w| w.eorste().set_bit().sofe().set_bit());
            // The DETACH bit in the UDCON register must be cleared so  the selected pull-up resistor is connected to the corresponding data line and the device is detected by the host.
            dev.udcon.modify(|_, w| w.detach().clear_bit());
        }
    });
}

#[avr_device::interrupt(atmega32u4)]
fn USB_GEN() {
    // The PLL is configured in the ISR of the USB controller as soon as a VBUS interrupt has been triggered.
    interrupt::free(|cs| {
        // for debug
        MY_B6.borrow(cs).borrow_mut().as_mut().unwrap().set_high();
        if let (Some(dev), Some(pll)) = (
            MY_USB.borrow(cs).borrow().as_ref(),
            MY_PLL.borrow(cs).borrow().as_ref(),
        ) {
            if dev.udint.read().eorsti().bit_is_set() {
                dev.udint.modify(|_, w| w.eorsti().clear_bit());
                // USB_STATE_RESET
                //let r = configure_endpoint(dev, 0);
                //if r == 0 {}
            }
            if dev.usbint.read().vbusti().bit_is_set() {
                dev.usbint.modify(|_, w| w.vbusti().clear_bit());
                if dev.usbsta.read().vbus().bit_is_set() {
                    pll.pllcsr.modify(|_, w| w.pindiv().set_bit()); // 16MHz
                    loop {
                        if pll.pllcsr.read().plock().bit_is_clear() {
                            break;
                        }
                    }
                    // USB_STATE_POWERED
                } else {
                    pll.pllcsr.modify(|_, w| unsafe { w.bits(0) });
                    // USB_STATE_UNATTACHED
                }
            }
        }
    });
}

#[avr_device::interrupt(atmega32u4)]
fn USB_COM() {}
