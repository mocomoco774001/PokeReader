use super::reader::Gen2Reader;
use crate::utils;

const DIV_INCREMENTS: [u8; 16] = [
    0x12, 0x12, 0x12, 0x13, 0x12, 0x12, 0x13, 0x12, 0x12, 0x13, 0x12, 0x12, 0x13, 0x12, 0x12, 0x13,
];

// Don't worry, I don't feel great about this either
// This is hacky while explorations are happening
static mut RNG_ADVANCE: u32 = 0;
static mut ADIV: u8 = 0;
static mut SDIV: u8 = 0;
static mut CYCLE_COUNTER: u32 = 0;
static mut HOOK_CALL_COUNT: u32 = 0;
static mut LAST_HOOK_R0: u32 = 0;
static mut LAST_HOOK_PC: u16 = 0;
static mut DIV_READ_COUNT: u32 = 0;
static mut LAST_DIV_READ_PC: u16 = 0;
static mut IO_READ_COUNTS: [u32; 128] = [0; 128];
static mut IO_LAST_READ_PC: [u16; 128] = [0; 128];

pub fn measured_div() -> u16 {
    unsafe { (ADIV as u16) << 8 | SDIV as u16 }
}

pub fn rng_advance() -> u32 {
    unsafe { RNG_ADVANCE }
}

pub fn reset_rng_advance() {
    unsafe { RNG_ADVANCE = 0 };
}

pub fn hook_call_count() -> u32 {
    unsafe { HOOK_CALL_COUNT }
}

pub fn last_hook_r0() -> u32 {
    unsafe { LAST_HOOK_R0 }
}

pub fn last_hook_pc() -> u16 {
    unsafe { LAST_HOOK_PC }
}

pub fn div_read_count() -> u32 {
    unsafe { DIV_READ_COUNT }
}

pub fn last_div_read_pc() -> u16 {
    unsafe { LAST_DIV_READ_PC }
}

pub fn io_read_count(index: u8) -> u32 {
    unsafe { IO_READ_COUNTS[index as usize] }
}

pub fn io_last_read_pc(index: u8) -> u16 {
    unsafe { IO_LAST_READ_PC[index as usize] }
}

// This isn't currently used, but it's been helpful
fn update_cycle_counter(regs: &[u32], _stack_pointer: *mut u32) {
    let cycle_counter = regs[0];
    unsafe { CYCLE_COUNTER = CYCLE_COUNTER.wrapping_add(cycle_counter) };
}

#[repr(C)]
pub struct DivTracker {
    last_div: u8,
    index: usize,
    correct_index: bool,
}

impl DivTracker {
    const fn new() -> Self {
        Self {
            last_div: 0,
            index: 0,
            correct_index: false,
        }
    }

    fn update(&mut self, div: u8) {
        let small_index = self.index % DIV_INCREMENTS.len();
        let diff = div.wrapping_sub(self.last_div);
        self.last_div = div;

        if diff != 0x12 && diff != 0x13 {
            self.correct_index = false;
        }

        if diff != DIV_INCREMENTS[small_index]
            && [2, 3, 5, 6, 8, 9].contains(&(small_index))
            && (self.index >= DIV_INCREMENTS.len() || self.correct_index)
        {
            self.index = match small_index {
                2 => 1 + 0x562,
                3 => 1 + 0x563,
                5 => 1 + 0x22b5,
                6 => 1 + 0x22b6,
                8 => 1 + 8,
                9 => 1 + 9,
                _ => 0,
            };
            self.correct_index = true;
        } else if diff != DIV_INCREMENTS[small_index] {
            self.index = 0;
            self.correct_index = false;
        } else {
            self.index = (self.index + 1) % 0x4000;
        }
    }

    pub fn index(&self) -> Option<usize> {
        // Hides until ready
        match self.correct_index {
            true => Some(self.index),
            false => None,
        }
    }
}

static mut ADD_DIV_TRACKER: DivTracker = DivTracker::new();
static mut SUB_DIV_TRACKER: DivTracker = DivTracker::new();

pub fn add_div_tracker() -> &'static DivTracker {
    unsafe { &ADD_DIV_TRACKER }
}

pub fn sub_div_tracker() -> &'static DivTracker {
    unsafe { &SUB_DIV_TRACKER }
}

fn gb_read_mem(regs: &[u32], _stack_pointer: *mut u32) {
    unsafe {
        HOOK_CALL_COUNT = HOOK_CALL_COUNT.wrapping_add(1);
        LAST_HOOK_R0 = regs[0];
    }
    let reader = Gen2Reader::crystal();
    let pc = reader.pc_reg();
    unsafe { LAST_HOOK_PC = pc; }
    if (0xff00..0xff80).contains(&(regs[0] as usize)) {
        let index = (regs[0] as usize - 0xff00) as usize;
        unsafe {
            IO_READ_COUNTS[index] = IO_READ_COUNTS[index].wrapping_add(1);
            IO_LAST_READ_PC[index] = pc;
        }
    }
    if regs[0] != 0xff04 {
        return;
    }

    let reader = Gen2Reader::crystal();
    let pc = reader.pc_reg();
    unsafe {
        DIV_READ_COUNT = DIV_READ_COUNT.wrapping_add(1);
        LAST_DIV_READ_PC = pc;
    }
    const RNG_DIV_READ_1: u16 = 0x2b5 + 1;
    const RNG_DIV_READ_2: u16 = 0x2bd + 1;
    if pc == RNG_DIV_READ_1 {
        let div = reader.div();
        unsafe { ADIV = div };

        unsafe { ADD_DIV_TRACKER.update(div) };
        unsafe { RNG_ADVANCE = RNG_ADVANCE.wrapping_add(1) };
    }
    if pc == RNG_DIV_READ_2 {
        let div = reader.div();
        unsafe { SDIV = div };
        unsafe { SUB_DIV_TRACKER.update(div) };
    }
}

pub fn init_crystal() {
    utils::hook_game_branch!(
        game_name = crystal,
        update_cycle_counter = 0x1a8360,
        gb_read_mem = 0x1af17c,
    );
}

pub fn init_crystal_jp() {
    // Japanese Crystal has a separate GB memory-read routine.
    // Hook the JP entry point used by game_lib::read_gb_mem_jp.
    utils::hook_game_branch!(
        game_name = crystal,
        gb_read_mem = 0x169018,
    );
}
