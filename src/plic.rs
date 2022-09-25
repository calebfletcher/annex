use conquer_once::spin::OnceCell;
use fdt::Fdt;

static PLIC: OnceCell<Plic> = OnceCell::uninit();

struct Plic {
    base_address: usize,
}

const ENABLE_OFFSET: usize = 0x2000;
const ENABLE_STRIDE: usize = 0x100;
const ENABLE_S_MODE_OFFSET: usize = 0x80;
const PENDING_OFFSET: usize = 0x1000;
const THRESHOLD_CLAIM_OFFSET: usize = 0x20_0000;
const THRESHOLD_CLAIM_S_MODE_OFFSET: usize = 0x1000;

/// Initialise the PLIC.
pub fn init(fdt: &Fdt) {
    let plic_node = fdt.find_node("/soc/plic").unwrap();
    let base_address = plic_node.reg().unwrap().next().unwrap().starting_address;
    PLIC.init_once(|| Plic {
        base_address: base_address as usize,
    });

    // Allow all interrupts through the PLIC
    set_threshold(0);

    // Enable the UART interrupt
    set_priority(0x0A, 1);
    set_enable(0, 0x0A, true);
}

pub fn set_priority(id: usize, priority: u8) {
    let addr = (PLIC.get().unwrap().base_address + 4 * id) as *mut u32;
    unsafe { addr.write_volatile(priority as u32) }
}

/// Check if a particular interrupt is pending.
#[allow(dead_code)]
pub fn interrupt_pending(id: usize) -> bool {
    let plic_base = PLIC.get().unwrap().base_address + PENDING_OFFSET;
    let offset = id / 32;
    let bit_index = id % 32;
    let addr = (plic_base + 4 * offset) as *mut u32;
    let reg = unsafe { addr.read_volatile() };
    reg & (1 << bit_index) != 0
}

/// Enable a particular interrupt.
pub fn set_enable(hart: usize, id: usize, enable: bool) {
    let plic_base = PLIC.get().unwrap().base_address
        + ENABLE_OFFSET
        + ENABLE_STRIDE * hart
        + ENABLE_S_MODE_OFFSET;
    let offset = id / 32;
    let bit_index = id % 32;
    let addr = (plic_base + 4 * offset) as *mut u32;

    let mask = 1 << bit_index;
    let bit = (enable as u32) << bit_index;

    unsafe {
        let current = addr.read_volatile();

        let new = (current & !mask) | bit;
        addr.write_volatile(new);
    }
}

/// Set the threshold required to trigger an interrupt.
pub fn set_threshold(threshold: u8) {
    let addr = (PLIC.get().unwrap().base_address
        + THRESHOLD_CLAIM_OFFSET
        + THRESHOLD_CLAIM_S_MODE_OFFSET) as *mut u32;

    unsafe {
        addr.write_volatile(threshold as u32);
    }
}

/// Try to claim an interrupt.
pub fn claim() -> Option<u32> {
    let addr = (PLIC.get().unwrap().base_address
        + THRESHOLD_CLAIM_OFFSET
        + THRESHOLD_CLAIM_S_MODE_OFFSET
        + 4) as *mut u32;

    let id = unsafe { addr.read_volatile() };

    if id != 0 {
        Some(id)
    } else {
        None
    }
}

/// Mark an interrupt as complete.
pub fn complete(id: u32) {
    let addr = (PLIC.get().unwrap().base_address
        + THRESHOLD_CLAIM_OFFSET
        + THRESHOLD_CLAIM_S_MODE_OFFSET
        + 4) as *mut u32;

    unsafe {
        addr.write_volatile(id);
    }
}

/// Disable the Clock Gate on the PLIC.
///
/// Documentation says this is needed, but isn't needed in QEMU. Leaving here for future platforms.
#[allow(dead_code)]
pub fn disable_clock_gate() {
    let clock_gate_reg = (PLIC.get().unwrap().base_address + 0x1F_F000) as *mut u32;
    unsafe {
        clock_gate_reg.write_volatile(1);
    }
}
