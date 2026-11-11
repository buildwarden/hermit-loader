mod console;

pub use self::console::Console;
pub mod drivers;
pub mod entry;
mod page_tables;
pub mod paging;

use core::arch::asm;
use core::ptr;

use aarch64_cpu::asm::barrier::{NSH, SY, dmb, dsb, isb};
use align_address::Align;
use fdt::Fdt;
use hermit_entry::Entry;
use hermit_entry::boot_info::{BootInfo, HardwareInfo, PlatformInfo, RawBootInfo, SerialPortBase};
use hermit_entry::elf::LoadedKernel;
use log::info;

use crate::arch::paging::*;
use crate::fdt_ext::FdtExt;
use crate::os::CONSOLE;
use crate::{BootInfoExt, stack};

/// start address of the RAM at Qemu's virt emulation
#[cfg(not(feature = "vz"))]
const RAM_START: u64 = 0x40000000;
/// start address of the RAM on Apple's Virtualization framework
#[cfg(feature = "vz")]
const RAM_START: u64 = 0x70000000;

/// The device tree the loader and the kernel use: the copy made by
/// [`get_memory`] once it exists, else the one the boot protocol passed in
/// `x0`, else start of RAM, where Qemu places it for ELF kernels
/// (see <https://qemu.readthedocs.io/en/latest/system/arm/virt.html>).
#[unsafe(link_section = ".data")]
static mut DEVICE_TREE_COPY: u64 = 0;

fn device_tree() -> u64 {
	let copy = unsafe { DEVICE_TREE_COPY };
	let boot = unsafe { entry::BOOT_DTB };
	if copy != 0 {
		copy
	} else if boot != 0 {
		boot
	} else {
		RAM_START
	}
}

fn fdt() -> Fdt<'static> {
	unsafe {
		Fdt::from_ptr(ptr::with_exposed_provenance(device_tree() as usize))
			.expect(".fdt file has invalid header")
	}
}

/// Returns where to load the kernel, after the loader.
///
/// The device tree is first copied to just below that address. The kernel
/// reserves all memory below itself, and the loader's page tables map it, so
/// the copy survives wherever the boot protocol placed the original (VZ puts
/// it near the top of RAM, unmapped and unreserved). The range skips the
/// initrd if it would overlap it, since the kernel is still read from there.
pub unsafe fn get_memory(memory_size: u64) -> u64 {
	let loader_end = elf_symbols::executable_end().expose_provenance() as u64;
	let fdt_size = fdt().total_size() as u64;

	let mut start = loader_end.align_up(BasePageSize::SIZE as u64);
	let need = fdt_size + LargePageSize::SIZE as u64 + memory_size;
	let initrd = find_kernel().as_ptr_range();
	let (initrd_start, initrd_end) = (initrd.start as u64, initrd.end as u64);
	if initrd_start < start + need && start < initrd_end {
		start = initrd_end.align_up(BasePageSize::SIZE as u64);
	}

	let copy = start;
	unsafe {
		ptr::copy_nonoverlapping(
			ptr::with_exposed_provenance::<u8>(device_tree() as usize),
			ptr::with_exposed_provenance_mut::<u8>(copy as usize),
			fdt_size as usize,
		);
		DEVICE_TREE_COPY = copy;
	}
	info!("Device tree copied to {copy:#x} ({fdt_size:#x} B)");

	(copy + fdt_size).align_up(LargePageSize::SIZE as u64)
}

pub fn find_kernel() -> &'static [u8] {
	fdt().find_kernel().unwrap()
}

#[allow(static_mut_refs)] // FIXME: disallow
pub unsafe fn boot_kernel(kernel_info: LoadedKernel) -> ! {
	let LoadedKernel {
		load_info,
		entry_point,
	} = kernel_info;

	let fdt = fdt();
	let cpus = fdt.cpus().count();
	info!("Detect {cpus} CPU(s)");

	// 0 when the device tree names no UART (VZ has none): nothing is mapped
	// and the kernel gets no serial port.
	let uart_address: u32 = CONSOLE.lock().get().get_stdout();
	info!("Detect UART at {uart_address:#x}");

	if uart_address != 0 {
		unsafe {
			page_tables::init(uart_address);
		}
		CONSOLE.lock().get().set_stdout(0x1000);
	}

	unsafe {
		page_tables::enable();
	}

	if let Some(device_type) = fdt
		.find_node("/memory")
		.and_then(|node| node.property("device_type"))
	{
		let device_type = core::str::from_utf8(device_type.value)
			.unwrap()
			.trim_matches(char::from(0));
		assert!(device_type == "memory");
	}
	info!("Memory found!");
	let regions = fdt.memory().regions().next().unwrap();
	let ram_start = regions.starting_address as u64;
	let ram_size = regions.size.unwrap() as u64;

	info!("ram_start: {ram_start:#x}, ram_size: {ram_size:#x}. Trying to jump into kernel soon.");
	let boot_info = BootInfo {
		hardware_info: HardwareInfo {
			phys_addr_range: ram_start..ram_start + ram_size,
			serial_port_base: if uart_address != 0 {
				SerialPortBase::new(0x1000)
			} else {
				None
			},
			device_tree: core::num::NonZeroU64::new(device_tree()),
		},
		load_info,
		platform_info: PlatformInfo::LinuxBoot,
	};

	let stack = stack::get_stack_ptr();
	let entry = ptr::with_exposed_provenance(entry_point.try_into().unwrap());
	let raw_boot_info = boot_info.write();

	unsafe { enter_kernel(stack, entry, raw_boot_info) }
}

unsafe fn enter_kernel(stack: *mut u8, entry: *const (), raw_boot_info: &'static RawBootInfo) -> ! {
	// Check expected signature of entry function
	let entry: Entry = {
		let entry: unsafe extern "C" fn(raw_boot_info: &'static RawBootInfo, cpu_id: u32) -> ! =
			unsafe { core::mem::transmute(entry) };
		entry
	};

	info!("Entering kernel at {entry:p}, stack at {stack:p}, raw_boot_info at {raw_boot_info:p}");

	// Memory barrier
	CONSOLE.lock().get().wait_empty();
	dsb(SY);
	isb(SY);
	dmb(SY);
	dsb(NSH);

	unsafe {
		asm!(
			"mov sp, {stack}",
			"br {entry}",
			stack = in(reg) stack,
			entry = in(reg) entry,
			in("x0") raw_boot_info,
			in("x1") 0,
			options(noreturn)
		)
	}
}
