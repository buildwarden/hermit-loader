use core::num::NonZeroU32;
use core::ptr;

use fdt::Fdt;

use crate::arch::aarch64::drivers::SerialDriver;
use crate::arch::drivers::qemu_serial::QemuSerial;
use crate::arch::drivers::xlnx_serial::XlnxSerial;
use crate::arch::drivers::{NullSerial, SerialPort, get_device};

pub struct Console {
	stdout: SerialPort,
}

fn stdout() -> SerialPort {
	/// Physical address of UART0 at Qemu's virt emulation
	const SERIAL_PORT_ADDRESS: u32 = 0x09000000;

	let fdt = unsafe {
		Fdt::from_ptr(ptr::with_exposed_provenance(super::device_tree() as usize))
			.expect(".fdt file has invalid header")
	};

	if let Some(port) = fdt.chosen().stdout().and_then(|node| get_device(node)) {
		return port;
	}
	// Without a stdout-path, fall back to Qemu's UART only if the machine
	// has a PL011 at all. Some hypervisors (Apple's Virtualization framework)
	// have no UART, and a store to an unbacked address stops the VM there,
	// so the loader stays silent instead.
	if fdt.find_compatible(&["arm,pl011"]).is_some() {
		SerialPort::Qemu(QemuSerial::from_addr(
			NonZeroU32::new(SERIAL_PORT_ADDRESS).unwrap(),
		))
	} else {
		SerialPort::Null(NullSerial)
	}
}

impl Console {
	pub fn write_bytes(&mut self, bytes: &[u8]) {
		self.stdout.putstr(bytes);
	}

	pub(super) fn get_stdout(&self) -> u32 {
		self.stdout.get_addr()
	}

	pub(crate) fn set_stdout(&mut self, stdout: u32) {
		match self.stdout {
			SerialPort::Qemu(_) => {
				self.stdout =
					SerialPort::Qemu(QemuSerial::from_addr(NonZeroU32::new(stdout).unwrap()))
			}
			SerialPort::Xlnx(_) => {
				self.stdout =
					SerialPort::Xlnx(XlnxSerial::from_addr(NonZeroU32::new(stdout).unwrap()))
			}
			SerialPort::Null(_) => {}
		}
		self.stdout.init();
	}

	pub(crate) fn wait_empty(&mut self) {
		self.stdout.wait_empty();
	}
}

impl Default for Console {
	fn default() -> Self {
		let stdout = stdout();
		Self { stdout }
	}
}

unsafe impl Send for Console {}
