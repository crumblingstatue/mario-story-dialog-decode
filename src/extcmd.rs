macro_rules! def {
    ($($id:literal $name:ident($($param:ident),*))*) => {
        pub fn n_params(id: u8) -> Option<u8> {
            match id {
                $($id => Some(${count($param)}) ,)*
                _ => None
            }
        }
        #[derive(Debug)]
        pub enum ExtCmd {
            $(
                $name{$($param: u8),*},
            )*
            Unknown(UnkCmd),
        }
        impl ExtCmd {
            pub fn from_id_and_args(id: u8, args: &[u8]) -> Option<Self> {
                Some(match id {
                    $($id => Self::$name{$($param: *args.get(${index()})?,)*},)*
                    _ => return None
                })
            }
        }
    };
}

pub struct UnkCmd(pub u8);

impl std::fmt::Debug for UnkCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:02X}", self.0)
    }
}

def! {
    0x05 TextColor(c)
    0x08 Unk8()
    0x0C AutoScroll(p1)
    0x0D FontSize(x, y)
    0x0E FontSizeReset()
    0x13 Unk13(p1)
    0x14 Unk14(p1)
    0x18 GraphicsB(p1, p2, p3, p4, p5, p6, p7)
    0x24 SaveTextColor()
    0x25 LoadTextColor()
    0x26 StartEffect(id)
    0x27 EndEffect(id)
    0x29 Unk29(p1)
    0x2F Voice(p1)
}

#[test]
fn test_n_params() {
    use std::assert_matches::assert_matches;
    assert_matches!(n_params(0x08), Some(0));
    assert_matches!(n_params(0x13), Some(1));
    assert_matches!(n_params(0x18), Some(7));
}

#[test]
fn test_from_id_and_args() {
    use std::assert_matches::assert_matches;
    assert_matches!(ExtCmd::from_id_and_args(0x08, &[]), Some(ExtCmd::Unk8 {}));
    assert_matches!(
        ExtCmd::from_id_and_args(0x18, &[1, 2, 3, 4, 5, 6, 7]),
        Some(ExtCmd::GraphicsB {
            p1: 1,
            p2: 2,
            p3: 3,
            p4: 4,
            p5: 5,
            p6: 6,
            p7: 7,
        })
    )
}
