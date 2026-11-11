// arm64 Linux Image header, for hypervisors that boot the loader through the
// Linux boot protocol (Documentation/arch/arm64/booting.rst), such as Apple's
// Virtualization framework. link-vz.ld places it at the start of the image.

.section .text.head, "ax"

.global _head
_head:
	b	_start		// code0: branch over the header
	.word	0		// code1
	.quad	0		// text_offset
	.quad	__image_size	// image_size, including .bss
	.quad	0b0010		// flags: little-endian, 4K pages, 2M-aligned at the base of RAM
	.quad	0		// res2
	.quad	0		// res3
	.quad	0		// res4
	.ascii	"ARM\x64"	// magic
	.word	0		// res5
