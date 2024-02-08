//! Instruction decoding and dispatch.

use read_fonts::tables::glyf::bytecode::Opcode;

use super::{Engine, HintError, HintErrorKind, Instruction};

/// Maximum number of instructions we will execute in `Engine::run()`. This
/// is used to ensure termination of a hinting program.
/// See <https://gitlab.freedesktop.org/freetype/freetype/-/blob/57617782464411201ce7bbc93b086c1b4d7d84a5/include/freetype/config/ftoption.h#L744>
const MAX_RUN_INSTRUCTIONS: usize = 1_000_000;

impl<'a> Engine<'a> {
    /// Decodes and dispatches all instructions until completion or error.
    pub fn run(&mut self) -> Result<(), HintError> {
        let mut count = 0;
        while let Some(ins) = self.decode() {
            let ins = ins?;
            self.dispatch(&ins)?;
            count += 1;
            if count > MAX_RUN_INSTRUCTIONS {
                return Err(HintError {
                    program: self.initial_program,
                    glyph_id: None,
                    pc: ins.pc,
                    kind: HintErrorKind::ExceededExecutionBudget,
                });
            }
        }
        Ok(())
    }

    /// Decodes the next instruction from the current program.
    pub fn decode(&mut self) -> Option<Result<Instruction<'a>, HintError>> {
        let ins = self.decoder.decode()?;
        Some(ins.map_err(|_| HintError {
            program: self.initial_program,
            glyph_id: None,
            pc: self.decoder.pc,
            kind: HintErrorKind::UnexpectedEndOfBytecode,
        }))
    }

    /// Executes the appropriate code for the given instruction.
    pub fn dispatch(&mut self, ins: &Instruction) -> Result<(), HintError> {
        let current_pc = self.decoder.pc;
        let current_program = self.initial_program;
        self.dispatch_inner(ins).map_err(|kind| HintError {
            program: current_program,
            glyph_id: None,
            pc: current_pc,
            kind,
        })
    }

    fn dispatch_inner(&mut self, ins: &Instruction) -> Result<(), HintErrorKind> {
        use Opcode::*;
        let opcode = ins.opcode;
        let raw_opcode = opcode as u8;
        match ins.opcode {
            SVTCA0 | SVTCA1 | SPVTCA0 | SPVTCA1 | SFVTCA0 | SFVTCA1 => self.op_svtca(raw_opcode)?,
            SPVTL0 | SPVTL1 | SFVTL0 | SFVTL1 => self.op_svtl(raw_opcode)?,
            SPVFS => self.op_spvfs()?,
            SFVFS => self.op_sfvfs()?,
            GPV => self.op_gpv()?,
            GFV => self.op_gfv()?,
            SFVTPV => self.op_sfvtpv()?,
            // ISECT => {}
            SRP0 => self.op_srp0()?,
            SRP1 => self.op_srp1()?,
            SRP2 => self.op_srp2()?,
            SZP0 => self.op_szp0()?,
            SZP1 => self.op_szp1()?,
            SZP2 => self.op_szp2()?,
            SZPS => self.op_szps()?,
            SLOOP => self.op_sloop()?,
            RTG => self.op_rtg()?,
            RTHG => self.op_rthg()?,
            SMD => self.op_smd()?,
            // ELSE => {}
            // JMPR => {}
            SCVTCI => self.op_scvtci()?,
            SSWCI => self.op_sswci()?,
            DUP => self.op_dup()?,
            POP => self.op_pop()?,
            CLEAR => self.op_clear()?,
            SWAP => self.op_swap()?,
            DEPTH => self.op_depth()?,
            CINDEX => self.op_cindex()?,
            MINDEX => self.op_mindex()?,
            // ALIGNPTS => {}
            // ? 0x28
            // UTP => {}
            // LOOPCALL => {}
            // CALL => {}
            // FDEF => {}
            // ENDF => {}
            // MDAP0 | MDAP1 => {}
            // IUP0 | IUP1 => {}
            // SHP0 | SHP1 => {}
            // SHC0 | SHC1 => {}
            // SHZ0 | SHZ1 => {}
            // SHPIX => {}
            // IP => {}
            // MSIRP0 | MISRP1 => {}
            // ALIGNRP => {}
            NPUSHB | NPUSHW => self.op_push(&ins.inline_operands)?,
            // WS => {}
            // RS => {}
            // WCVTP => {}
            // RCVT => {}
            // SCFS => {}
            // MD0 | MD1 => {}
            // MPPEM => {}
            // MPS => {}
            FLIPON => self.op_flipon()?,
            FLIPOFF => self.op_flipoff()?,
            // DEBUG => {}
            LT => self.op_lt()?,
            LTEQ => self.op_lteq()?,
            GT => self.op_gt()?,
            GTEQ => self.op_gteq()?,
            EQ => self.op_eq()?,
            NEQ => self.op_neq()?,
            ODD => self.op_odd()?,
            EVEN => self.op_even()?,
            // IF => {}
            // EIF => {}
            AND => self.op_and()?,
            OR => self.op_or()?,
            NOT => self.op_not()?,
            // DELTAP1 => {}
            SDB => self.op_sdb()?,
            SDS => self.op_sds()?,
            ADD => self.op_add()?,
            SUB => self.op_sub()?,
            DIV => self.op_div()?,
            MUL => self.op_mul()?,
            ABS => self.op_abs()?,
            NEG => self.op_neg()?,
            FLOOR => self.op_floor()?,
            CEILING => self.op_ceiling()?,
            // ROUND00 | ROUND01 | ROUND10 | ROUND11 => {}
            // "No round" means do nothing :)
            NROUND00 | NROUND01 | NROUND10 | NROUND11 => {}
            // WCVTF => {}
            // DELTAP2 | DELTAP3 => {}
            // DELTAC1 | DELTAC2 | DELTAC3 => {}
            SROUND => self.op_sround()?,
            S45ROUND => self.op_s45round()?,
            // JROT => {}
            // JROF => {}
            ROFF => self.op_roff()?,
            // ? 0x7B
            RUTG => self.op_rutg()?,
            RDTG => self.op_rdtg()?,
            SANGW => self.op_sangw()?,
            // Unsupported instruction, do nothing
            AA => {}
            // FLIPPT => {}
            // FLIPRGON => {}
            // FLIPRGOFF => {}
            // ? 0x83 | 0x84
            SCANCTRL => self.op_scanctrl()?,
            SDPVTL0 | SDPVTL1 => self.op_sdpvtl(raw_opcode)?,
            // GETINFO => {}
            // IDEF => {}
            ROLL => self.op_roll()?,
            MAX => self.op_max()?,
            MIN => self.op_min()?,
            SCANTYPE => self.op_scantype()?,
            // ? 0x8F | 0x90 (ADJUST?)
            // GETVARIATION => {}
            // GETDATA => {}
            _ => {
                // FreeType handles MIRP, MDRP and pushes here.
                // <https://gitlab.freedesktop.org/freetype/freetype/-/blob/57617782464411201ce7bbc93b086c1b4d7d84a5/src/truetype/ttinterp.c#L7629>
                // if opcode >= MIRP00000 {
                //     self.op_mirp(raw_opcode)?
                // } else if opcode >= MDRP00000 {
                //     self.op_mdrp(raw_opcode)?
                // } else
                if opcode >= PUSHB000 {
                    self.op_push(&ins.inline_operands)?;
                } else {
                    return Err(HintErrorKind::UnhandledOpcode(opcode));
                }
            }
        }
        Ok(())
    }
}