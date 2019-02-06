use spin::Mutex;
use riscv_decode::Instruction;
use crate::{csr, pmap};

#[allow(unused)]
mod constants {
    pub const TVEC_MODE: usize = 0x3;
    pub const TVEC_BASE: usize = !TVEC_MODE;

    pub const STATUS_UIE: usize = 1 << 0;
    pub const STATUS_SIE: usize = 1 << 1;
    pub const STATUS_UPIE: usize = 1 << 4;
    pub const STATUS_SPIE: usize = 1 << 5;
    pub const STATUS_SPP: usize = 1 << 8;
    pub const STATUS_FS0: usize = 1 << 13;
    pub const STATUS_FS1: usize = 1 << 14;
    pub const STATUS_XS0: usize = 1 << 15;
    pub const STATUS_XS1: usize = 1 << 16;
    pub const STATUS_SUM: usize = 1 << 18;
    pub const STATUS_MXR: usize = 1 << 19;
    pub const STATUS_SD: usize = 1 << 31; // Only for RV32!

    pub const IP_SSIP: usize = 1 << 1;
    pub const IP_STIP: usize = 1 << 5;
    pub const IP_SEIP: usize = 1 << 9;

    pub const IE_SSIE: usize = 1 << 1;
    pub const IE_STIE: usize = 1 << 5;
    pub const IE_SEIE: usize = 1 << 9;

    pub const MSTACK_BASE: usize = 0x80100000 - 16*4;
    pub const SSTACK_BASE: usize = 0x80200000 - 32*4;
}
use self::constants::*;

pub const MAX_TSTACK_ADDR: usize = 0x80200000;

trait UsizeBits {
    fn get(&self, mask: Self) -> bool;
    fn set(&mut self, mask: Self, value: bool);
}
impl UsizeBits for usize {
    fn get(&self, mask: Self) -> bool {
        *self & mask != 0
    }
    fn set(&mut self, mask: Self, value: bool) {
        if value {
            *self |= mask;
        } else {
            *self &= !mask;
        }
    }
}

// 0x340 = mscratch
// 0x140 = sscratch

#[naked]
pub unsafe fn mtrap_entry() -> ! {
    asm!(".align 4
          csrw 0x340, sp
          li sp, 0x80100000
          addi sp, sp, -16*4
          sw ra, 0*4(sp)
          sw t0, 1*4(sp)
          sw t1, 2*4(sp)
          sw t2, 3*4(sp)
          sw t3, 4*4(sp)
          sw t4, 5*4(sp)
          sw t5, 6*4(sp)
          sw t6, 7*4(sp)
          sw a0, 8*4(sp)
          sw a1, 9*4(sp)
          sw a2, 10*4(sp)
          sw a3, 11*4(sp)
          sw a4, 12*4(sp)
          sw a5, 13*4(sp)
          sw a6, 14*4(sp)
          sw a7, 15*4(sp)

          jal ra, mtrap

          lw ra, 0*4(sp)
          lw t0, 1*4(sp)
          lw t1, 2*4(sp)
          lw t2, 3*4(sp)
          lw t3, 4*4(sp)
          lw t4, 5*4(sp)
          lw t5, 6*4(sp)
          lw t6, 7*4(sp)
          lw a0, 8*4(sp)
          lw a1, 9*4(sp)
          lw a2, 10*4(sp)
          lw a3, 11*4(sp)
          lw a4, 12*4(sp)
          lw a5, 13*4(sp)
          lw a6, 14*4(sp)
          lw a7, 15*4(sp)
          csrr sp, 0x340
          mret" :::: "volatile");

    unreachable!()
}

#[no_mangle]
pub unsafe fn mtrap() {
    let cause = csrr!(mcause);
    match ((cause as isize) < 0, cause & 0xff) {
        (true, 0...3) => println!("software interrupt"),
        (true, 4...7) => println!("timer interrupt"),
        (true, 8...11) => println!("external interrupt"),
        (true, _) => println!("reserved interrupt"),
        (false, 0) => println!("instruction address misaligned"),
        (false, 1) => {
            println!("instruction access fault @ {:8x}", csrr!(mepc))
        }
        (false, 2) => println!("illegal instruction: {:x}", csrr!(mepc)),
        (false, 3) => println!("breakpoint"),
        (false, 4) => println!("load address misaligned"),
        (false, 5) => println!("load access fault"),
        (false, 6) => println!("store/AMO address misaligned"),
        (false, 7) => println!("store/AMO access fault"),
        (false, 8...11) => {
            println!("environment call");
            csrw!(mepc, csrr!(mepc) + 4);
            return;
        }
        (false, 12) => println!("instruction page fault"),
        (false, 13) => println!("load page fault"),
        (false, 14) => println!("reserved exception #14"),
        (false, 15) => println!("store/AMO page fault"),
        (false, _) => println!("reserved exception"),
    }

    loop {}
}

#[naked]
pub unsafe fn strap_entry() -> ! {
    asm!(".align 4
          csrw 0x140, sp
          li sp, 0x80200000
          addi sp, sp, -32*4

          sw ra, 1*4(sp)
          sw gp, 3*4(sp)
          sw tp, 4*4(sp)
          sw t0, 5*4(sp)
          sw t1, 6*4(sp)
          sw t2, 7*4(sp)
          sw s0, 8*4(sp)
          sw s1, 9*4(sp)
          sw a0, 10*4(sp)
          sw a1, 11*4(sp)
          sw a2, 12*4(sp)
          sw a3, 13*4(sp)
          sw a4, 14*4(sp)
          sw a5, 15*4(sp)
          sw a6, 16*4(sp)
          sw a7, 17*4(sp)
          sw s2, 18*4(sp)
          sw s3, 19*4(sp)
          sw s4, 20*4(sp)
          sw s5, 21*4(sp)
          sw s6, 22*4(sp)
          sw s7, 23*4(sp)
          sw s8, 24*4(sp)
          sw s9, 25*4(sp)
          sw s10, 26*4(sp)
          sw s11, 27*4(sp)
          sw t3, 28*4(sp)
          sw t4, 29*4(sp)
          sw t5, 30*4(sp)
          sw t6, 31*4(sp)

          jal ra, strap

          lw ra, 1*4(sp)
          lw gp, 3*4(sp)
          lw tp, 4*4(sp)
          lw t0, 5*4(sp)
          lw t1, 6*4(sp)
          lw t2, 7*4(sp)
          lw s0, 8*4(sp)
          lw s1, 9*4(sp)
          lw a0, 10*4(sp)
          lw a1, 11*4(sp)
          lw a2, 12*4(sp)
          lw a3, 13*4(sp)
          lw a4, 14*4(sp)
          lw a5, 15*4(sp)
          lw a6, 16*4(sp)
          lw a7, 17*4(sp)
          lw s2, 18*4(sp)
          lw s3, 19*4(sp)
          lw s4, 20*4(sp)
          lw s5, 21*4(sp)
          lw s6, 22*4(sp)
          lw s7, 23*4(sp)
          lw s8, 24*4(sp)
          lw s9, 25*4(sp)
          lw s10, 26*4(sp)
          lw s11, 27*4(sp)
          lw t3, 28*4(sp)
          lw t4, 29*4(sp)
          lw t5, 30*4(sp)
          lw t6, 31*4(sp)
          csrr sp, 0x140
          sret" :::: "volatile");

    unreachable!()
}

// struct TrapFrame {
//     ra: u32,
//     t0: u32,
//     t1: u32,
//     t2: u32,
//     t3: u32,
//     t4: u32,
//     t5: u32,
//     t6: u32,
//     a0: u32,
//     a1: u32,
//     a2: u32,
//     a3: u32,
//     a4: u32,
//     a5: u32,
//     a6: u32,
//     a7: u32,
// }

#[derive(Default)]
pub struct ShadowState {
    // sedeleg: usize, -- Hard-wired to zero
    // sideleg: usize, -- Hard-wired to zero

    sstatus: usize,
    sie: usize,
    // sip: usize, -- checked dynamically on read
    stvec: usize,
    // scounteren: usize, -- Hard-wired to zero
    sscratch: usize,
    sepc: usize,
    scause: usize,
    stval: usize,
    satp: usize,

    smode: bool
}
impl ShadowState {
    pub const fn new() -> Self {
        Self {
            sstatus: 0,
            stvec: 0,
            sie: 0,
            sscratch: 0,
            sepc: 0,
            scause: 0,
            stval: 0,
            satp: 0,

            smode: true,
        }
    }
    pub fn push_sie(&mut self) {
        self.sstatus.set(STATUS_SPIE, self.sstatus.get(STATUS_SIE));
        self.sstatus.set(STATUS_SIE, false);
    }
    pub fn pop_sie(&mut self) {
        self.sstatus.set(STATUS_SIE, self.sstatus.get(STATUS_SPIE));
        self.sstatus.set(STATUS_SPIE, true);
    }

    pub fn get_csr(&self, csr: u32) -> Option<usize> {
        Some(match csr as usize {
            csr::sstatus => self.sstatus,
            csr::satp => self.satp,
            csr::sie => self.sie,
            csr::stvec => self.stvec,
            csr::sscratch => self.sscratch,
            csr::sepc => self.sepc,
            csr::scause => self.scause,
            csr::stval => self.stval,
            csr::sip => csrr!(sip),
            csr::sedeleg => 0,
            csr::sideleg => 0,
            csr::scounteren => 0,
            _ => return None,
        })
    }

    pub fn set_csr(&mut self, csr: u32, value: usize) -> bool {
        match csr as usize {
            csr::sstatus => {
                self.sstatus = value;
                // TODO: apply any side effects
            }
            csr::satp => {
                self.satp = value;
                // TODO: apply any side effects
            }
            csr::sie => self.sie = value & (IE_SEIE | IE_STIE | IE_SSIE),
            csr::stvec => self.stvec = value & !0x2,
            csr::sscratch => self.sscratch = value,
            csr::sepc => self.sepc = value,
            csr::scause => self.scause = value,
            csr::stval => self.stval = value,
            csr::sip => csrs!(sip, value & IP_SSIP),
            csr::sedeleg |
            csr::sideleg |
            csr::scounteren => {}
            _ => return false,
        }

        return true;
    }
}

static SHADOW_STATE: Mutex<ShadowState> = Mutex::new(ShadowState::new());

#[no_mangle]
pub unsafe fn strap() {
    let cause = csrr!(scause);
    let status = csrr!(sstatus);

    if status.get(STATUS_SPP) {
        println!("Trap from within hypervisor?!");
        println!("sepc = {:#x}", csrr!(sepc));
        println!("cause = {}", cause);
        loop {}
    }

    let mut state = SHADOW_STATE.lock();
    if (cause as isize) < 0 {
        let enabled = state.sstatus.get(STATUS_SIE);
        let unmasked = state.sie & (1 << (cause & 0xff)) != 0;
        if (!state.smode || enabled) && unmasked {
            forward_interrupt(&mut state, cause, csrr!(sepc));
        }
    } else if cause == 12 || cause == 13 || cause == 15 {
        // TODO: Handle page fault
    } else if cause == 2 && state.smode {
        let pc = csrr!(sepc);
        let il = *(pc as *const u16);
        let len = riscv_decode::instruction_length(il);
        let instruction = match len {
            2 => il as u32,
            4 => il as u32 | ((*((pc + 2) as *const u16) as u32) << 16),
            _ => unreachable!(),
        };

        match riscv_decode::try_decode(instruction) {
            Some(Instruction::Sret) => {
                state.pop_sie();
                state.smode = state.sstatus.get(STATUS_SPP);
                state.sstatus.set(STATUS_SPP, false);
                csrw!(sepc, state.sepc);
            }
            Some(fence @ Instruction::SfenceVma(_)) => pmap::handle_sfence_vma(&mut state, fence),
            Some(Instruction::Csrrw(i)) => if let Some(prev) = state.get_csr(i.csr()) {
                state.set_csr(i.csr(), get_register(i.rs1()));
                set_register(i.rd(), prev);
            }
            Some(Instruction::Csrrs(i)) => if let Some(prev) = state.get_csr(i.csr()) {
                state.set_csr(i.csr(), prev | get_register(i.rs1()));
                set_register(i.rd(), prev);
            }
            Some(Instruction::Csrrc(i)) => if let Some(prev) = state.get_csr(i.csr()) {
                state.set_csr(i.csr(), prev & !get_register(i.rs1()));
                set_register(i.rd(), prev);
            }
            Some(Instruction::Csrrwi(i)) => if let Some(prev) = state.get_csr(i.csr()) {
                state.set_csr(i.csr(), i.zimm() as usize);
                set_register(i.rd(), prev);
            }
            Some(Instruction::Csrrsi(i)) => if let Some(prev) = state.get_csr(i.csr()) {
                state.set_csr(i.csr(), prev | (i.zimm() as usize));
                set_register(i.rd(), prev);
            }
            Some(Instruction::Csrrci(i)) => if let Some(prev) = state.get_csr(i.csr()) {
                state.set_csr(i.csr(), prev & !(i.zimm() as usize));
                set_register(i.rd(), prev);
            }
            _ => forward_exception(&mut state, cause, pc),
        }
        csrw!(sepc, pc + len);
    } else if cause == 8 && state.smode {
        println!("SMode");
        asm!("
          lw a0, 10*4($0)
          lw a1, 11*4($0)
          lw a2, 12*4($0)
          lw a3, 13*4($0)
          lw a4, 14*4($0)
          lw a5, 15*4($0)
          lw a6, 16*4($0)
          lw a7, 17*4($0)
          ecall
          sw a7, 17*4($0)"
             :: "r"(SSTACK_BASE) : "a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7": "volatile");
        csrw!(sepc, csrr!(sepc) + 4);
    } else {
        forward_exception(&mut state, cause, csrr!(sepc));
    }
}

fn forward_interrupt(state: &mut ShadowState, cause: usize, sepc: usize) {
    state.push_sie();
    state.sepc = sepc;
    state.scause = cause;
    state.sstatus.set(STATUS_SPP, state.smode);
    state.stval = 0;
    state.smode = true;

    match state.stvec & TVEC_MODE {
        0 => csrw!(sepc, state.stvec & TVEC_BASE),
        1 => csrw!(sepc, (state.stvec & TVEC_BASE) + 4 * cause & 0xff),
        _ => unreachable!(),
    }
}

fn forward_exception(state: &mut ShadowState, cause: usize, sepc: usize) {
    state.push_sie();
    state.sepc = sepc;
    state.scause = cause;
    state.sstatus.set(STATUS_SPP, state.smode);
    state.stval = csrr!(stval);
    state.smode = true;
    csrw!(sepc, state.stvec & TVEC_BASE);
}

fn set_register(reg: u32, value: usize) {
    match reg {
        0 => {},
        1 | 3..=31 => unsafe { *(SSTACK_BASE as *mut u32).offset(reg as isize) = value as u32; }
        2 => csrw!(sscratch, value),
        _ => unreachable!(),
    }
}
fn get_register(reg: u32) -> usize {
    match reg {
        0 => 0,
        1 | 3..=31 => unsafe { *(SSTACK_BASE as *const u32).offset(reg as isize) as usize },
        2 => csrr!(sscratch),
        _ => unreachable!(),
    }
}
