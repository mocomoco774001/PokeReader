use crate::utils::game_fn;

game_fn!(read_gb_mem_other(gb_addr: u32) -> u8 = 0x1690b0);
game_fn!(read_gb_mem_jp(gb_addr: u32) -> u8 = 0x169018);

pub fn read_gb_mem(gb_addr: u32) -> u8 {
    match crate::title::loaded_title() {
        Ok(crate::title::LoadedTitle::CrystalJp) => read_gb_mem_jp(gb_addr),
        _ => read_gb_mem_other(gb_addr),
    }
}

pub mod gb_mem {
    use super::*;

    pub fn read_u8(addr: u32) -> u8 {
        read_gb_mem(addr)
    }

    pub fn read_u16(addr: u32) -> u16 {
        (read_u8(addr) as u16) << 8 | read_u8(addr + 1) as u16
    }

    pub fn read_u32(addr: u32) -> u32 {
        (read_u16(addr) as u32) << 16 | read_u16(addr + 2) as u32
    }
}
