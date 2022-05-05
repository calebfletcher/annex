use crate::{apic, gdt, hlt_loop, println, serial_println, threading};
use lazy_static::lazy_static;
use x86_64::{
    instructions::port::Port,
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    APIC = 61,
    IOAPICKB = 91,
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        x86_64::set_general_handler!(&mut idt, general_handler);

        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::APIC as usize].set_handler_fn(apic_interrupt_handler);
        idt[InterruptIndex::IOAPICKB as usize].set_handler_fn(ioapic_keyboard_interrupt_handler);

        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

fn general_handler(_stack_frame: InterruptStackFrame, index: u8, _error_code: Option<u64>) {
    println!("handle irq {}", index)
}

extern "x86-interrupt" fn ioapic_keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe { apic::LAPIC.try_get().unwrap().lock().end_of_interrupt() };
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);
}

extern "x86-interrupt" fn apic_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe { apic::LAPIC.try_get().unwrap().lock().end_of_interrupt() };

    unsafe { threading::schedule() };
}

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, code: u64) {
    serial_println!("EXCEPTION: INVALID TSS({})\n{:#?}", code, stack_frame);
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: INVALID HANDLER\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    code: u64,
) {
    serial_println!(
        "EXCEPTION: GENERAL PROTECTION ({})\n{:#?}",
        code,
        stack_frame
    );
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "EXCEPTION: DOUBLE FAULT ({})\n{:#?}",
        error_code, stack_frame
    );
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
    serial_println!("hello");
}
