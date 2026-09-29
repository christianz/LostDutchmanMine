//! The trace hash layout, checked against values computed independently in Python.

use machine::Machine;

#[test]
fn reset_machine_hash() {
    assert_eq!(Machine::new().state_hash(), 0xaeea_d64c_1e43_1b8d);
}

#[test]
fn hash_covers_registers_memory_and_palette_in_order() {
    let mut m = Machine::new();
    let r = &mut m.regs;
    (r.ax, r.bx, r.cx, r.dx, r.si, r.di, r.bp) = (0x1234, 2, 3, 4, 5, 6, 7);
    (r.sp, r.cs, r.ds, r.es, r.ss, r.ip, r.flags) =
        (0xfffe, 0x1000, 0x0ff0, 0x0ff0, 0x2000, 0xca, 0x246);
    m.memory.as_bytes_mut()[0x12345] = 0xab;
    m.memory.as_bytes_mut()[0xfffff] = 1;
    m.vga.palette[7] = 0xff10_2030;
    assert_eq!(m.state_hash(), 0xce67_606a_4292_3a60);
}
