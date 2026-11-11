/*
 * Linker script for the aarch64 loader booted as an arm64 Linux Image, as
 * Apple's Virtualization framework (VZ) does with VZLinuxBootLoader.
 *
 * VZ's RAM starts at 0x70000000 and it places the image, with text_offset 0,
 * at the start of RAM. The Image header (head.s) must be the first bytes of
 * the flat binary, so .text.head comes first and the ELF headers are not
 * part of any loaded segment. .bss is laid out in the file's address range so
 * image_size covers it and VZ places nothing over it.
 */

ENTRY(_start)

PHDRS
{
	text PT_LOAD FLAGS(5);
	data PT_LOAD FLAGS(6);
}

SECTIONS
{
	. = 0x70000000;
	__executable_start = .;

	.text : {
		KEEP(*(.text.head))
		*(.text .text.*)
	} :text
	_etext = .;

	.rodata : { *(.rodata .rodata.*) } :text
	.eh_frame_hdr : { *(.eh_frame_hdr) } :text
	.eh_frame : { KEEP(*(.eh_frame)) } :text

	. = ALIGN(0x1000);
	.data : { *(.data .data.*) } :data
	.got : { *(.got .got.*) } :data
	_edata = .;

	.bss : { *(.bss .bss.*) *(COMMON) } :data
	. = ALIGN(0x1000);
	_end = .;

	__image_size = _end - __executable_start;
}
