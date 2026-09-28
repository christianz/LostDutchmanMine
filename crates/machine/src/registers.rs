//! The 8086 register file.

/// The fourteen 16-bit registers, including the instruction pointer and flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Registers {
    /// Accumulator.
    pub ax: u16,
    /// Base.
    pub bx: u16,
    /// Count.
    pub cx: u16,
    /// Data.
    pub dx: u16,
    /// Source index.
    pub si: u16,
    /// Destination index.
    pub di: u16,
    /// Base pointer.
    pub bp: u16,
    /// Stack pointer.
    pub sp: u16,
    /// Code segment.
    pub cs: u16,
    /// Data segment.
    pub ds: u16,
    /// Extra segment.
    pub es: u16,
    /// Stack segment.
    pub ss: u16,
    /// Instruction pointer.
    pub ip: u16,
    /// Flags; see [`crate::Flag`].
    pub flags: u16,
}

/// A 16-bit general, index, pointer or segment register.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reg16 {
    /// Accumulator.
    Ax,
    /// Count.
    Cx,
    /// Data.
    Dx,
    /// Base.
    Bx,
    /// Stack pointer.
    Sp,
    /// Base pointer.
    Bp,
    /// Source index.
    Si,
    /// Destination index.
    Di,
    /// Extra segment.
    Es,
    /// Code segment.
    Cs,
    /// Stack segment.
    Ss,
    /// Data segment.
    Ds,
}

/// An 8-bit half of AX, BX, CX or DX.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reg8 {
    /// Low byte of AX.
    Al,
    /// Low byte of CX.
    Cl,
    /// Low byte of DX.
    Dl,
    /// Low byte of BX.
    Bl,
    /// High byte of AX.
    Ah,
    /// High byte of CX.
    Ch,
    /// High byte of DX.
    Dh,
    /// High byte of BX.
    Bh,
}

impl Reg8 {
    /// The 16-bit register this half belongs to, and whether it is the high byte.
    pub const fn parent(self) -> (Reg16, bool) {
        match self {
            Reg8::Al => (Reg16::Ax, false),
            Reg8::Cl => (Reg16::Cx, false),
            Reg8::Dl => (Reg16::Dx, false),
            Reg8::Bl => (Reg16::Bx, false),
            Reg8::Ah => (Reg16::Ax, true),
            Reg8::Ch => (Reg16::Cx, true),
            Reg8::Dh => (Reg16::Dx, true),
            Reg8::Bh => (Reg16::Bx, true),
        }
    }
}

impl Registers {
    /// Reads a 16-bit register.
    pub const fn get(&self, register: Reg16) -> u16 {
        match register {
            Reg16::Ax => self.ax,
            Reg16::Cx => self.cx,
            Reg16::Dx => self.dx,
            Reg16::Bx => self.bx,
            Reg16::Sp => self.sp,
            Reg16::Bp => self.bp,
            Reg16::Si => self.si,
            Reg16::Di => self.di,
            Reg16::Es => self.es,
            Reg16::Cs => self.cs,
            Reg16::Ss => self.ss,
            Reg16::Ds => self.ds,
        }
    }

    /// Writes a 16-bit register.
    pub const fn set(&mut self, register: Reg16, value: u16) {
        let slot = match register {
            Reg16::Ax => &mut self.ax,
            Reg16::Cx => &mut self.cx,
            Reg16::Dx => &mut self.dx,
            Reg16::Bx => &mut self.bx,
            Reg16::Sp => &mut self.sp,
            Reg16::Bp => &mut self.bp,
            Reg16::Si => &mut self.si,
            Reg16::Di => &mut self.di,
            Reg16::Es => &mut self.es,
            Reg16::Cs => &mut self.cs,
            Reg16::Ss => &mut self.ss,
            Reg16::Ds => &mut self.ds,
        };
        *slot = value;
    }

    /// Reads an 8-bit register.
    pub const fn get8(&self, register: Reg8) -> u8 {
        let (parent, high) = register.parent();
        let [low, top] = self.get(parent).to_le_bytes();
        if high { top } else { low }
    }

    /// Writes an 8-bit register, leaving the other half unchanged.
    pub const fn set8(&mut self, register: Reg8, value: u8) {
        let (parent, high) = register.parent();
        let [low, top] = self.get(parent).to_le_bytes();
        let bytes = if high { [low, value] } else { [value, top] };
        self.set(parent, u16::from_le_bytes(bytes));
    }

    /// AL.
    pub const fn al(&self) -> u8 {
        self.get8(Reg8::Al)
    }
    /// AH.
    pub const fn ah(&self) -> u8 {
        self.get8(Reg8::Ah)
    }
    /// BL.
    pub const fn bl(&self) -> u8 {
        self.get8(Reg8::Bl)
    }
    /// BH.
    pub const fn bh(&self) -> u8 {
        self.get8(Reg8::Bh)
    }
    /// CL.
    pub const fn cl(&self) -> u8 {
        self.get8(Reg8::Cl)
    }
    /// CH.
    pub const fn ch(&self) -> u8 {
        self.get8(Reg8::Ch)
    }
    /// DL.
    pub const fn dl(&self) -> u8 {
        self.get8(Reg8::Dl)
    }
    /// DH.
    pub const fn dh(&self) -> u8 {
        self.get8(Reg8::Dh)
    }
    /// Sets AL.
    pub const fn set_al(&mut self, value: u8) {
        self.set8(Reg8::Al, value);
    }
    /// Sets AH.
    pub const fn set_ah(&mut self, value: u8) {
        self.set8(Reg8::Ah, value);
    }
    /// Sets BL.
    pub const fn set_bl(&mut self, value: u8) {
        self.set8(Reg8::Bl, value);
    }
    /// Sets BH.
    pub const fn set_bh(&mut self, value: u8) {
        self.set8(Reg8::Bh, value);
    }
    /// Sets CL.
    pub const fn set_cl(&mut self, value: u8) {
        self.set8(Reg8::Cl, value);
    }
    /// Sets CH.
    pub const fn set_ch(&mut self, value: u8) {
        self.set8(Reg8::Ch, value);
    }
    /// Sets DL.
    pub const fn set_dl(&mut self, value: u8) {
        self.set8(Reg8::Dl, value);
    }
    /// Sets DH.
    pub const fn set_dh(&mut self, value: u8) {
        self.set8(Reg8::Dh, value);
    }

    /// The registers in the order the trace hash covers them.
    pub const fn in_trace_order(&self) -> [u16; 14] {
        [
            self.ax, self.bx, self.cx, self.dx, self.si, self.di, self.bp, self.sp, self.cs,
            self.ds, self.es, self.ss, self.ip, self.flags,
        ]
    }

    /// The registers from [`Registers::in_trace_order`].
    pub const fn from_trace_order(values: [u16; 14]) -> Self {
        let [ax, bx, cx, dx, si, di, bp, sp, cs, ds, es, ss, ip, flags] = values;
        Registers { ax, bx, cx, dx, si, di, bp, sp, cs, ds, es, ss, ip, flags }
    }
}
