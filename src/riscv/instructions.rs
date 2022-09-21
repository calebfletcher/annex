/// Calculate the size of an instruction at an address
pub fn instruction_size(instruction: usize) -> usize {
    let opcode = unsafe { *(instruction as *const u8) & 0b11 };
    match opcode {
        0..=2 => 2,
        3 => 4,
        4.. => unreachable!(),
    }
}
