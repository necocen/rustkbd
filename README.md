# rustkbd

Keyboard firmware written in Rust.

### rustkbd-core
Abstraction layer for handling USB HID communication and left-right communication on split keyboard. This crate depends only `embedded-hal` and some other abstract crates, and does not depend on any specific boards/chips.

### necoboard-petit
Firmware implementation for 8 key split keyboard using RasPi-pico.

## Features

- Customizable key mapping
- Split keyboard support
- Layers support
- Media keys support

## TODOs

- [ ] Testing
- [ ] Better error handling
- [ ] Documentation
