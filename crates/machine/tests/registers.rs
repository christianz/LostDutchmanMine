//! Byte registers alias the halves of AX, BX, CX and DX.

use machine::{Reg8, Registers};

#[test]
fn byte_accessors_read_and_write_the_right_half() {
    let mut r =
        Registers { ax: 0x1234, bx: 0x5678, cx: 0x9abc, dx: 0xdef0, ..Registers::default() };
    assert_eq!((r.al(), r.ah(), r.bl(), r.bh()), (0x34, 0x12, 0x78, 0x56));
    assert_eq!((r.cl(), r.ch(), r.dl(), r.dh()), (0xbc, 0x9a, 0xf0, 0xde));
    r.set_al(0xaa);
    r.set_bh(0xbb);
    r.set_cl(0xcc);
    r.set_dh(0xdd);
    assert_eq!((r.ax, r.bx, r.cx, r.dx), (0x12aa, 0xbb78, 0x9acc, 0xddf0));
    r.set_ah(1);
    r.set_bl(2);
    r.set_ch(3);
    r.set_dl(4);
    assert_eq!((r.ax, r.bx, r.cx, r.dx), (0x01aa, 0xbb02, 0x03cc, 0xdd04));
}

#[test]
fn named_accessors_agree_with_the_enum() {
    let r = Registers { ax: 0x0102, bx: 0x0304, cx: 0x0506, dx: 0x0708, ..Registers::default() };
    let named = [r.al(), r.cl(), r.dl(), r.bl(), r.ah(), r.ch(), r.dh(), r.bh()];
    let by_enum = [Reg8::Al, Reg8::Cl, Reg8::Dl, Reg8::Bl, Reg8::Ah, Reg8::Ch, Reg8::Dh, Reg8::Bh]
        .map(|register| r.get8(register));
    assert_eq!(named, by_enum);
}
