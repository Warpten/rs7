use std::fmt;

use bytes::Buf;

use crate::lua::bytecode::{EndianBuffer, primitives::read_cstring};

pub mod variable {
    use std::{fmt, ops::Range};

    use bytes::Buf;

    use crate::{lua::bytecode::primitives::read_cstring, utils::ReadVar};

    #[repr(u8)]
    #[derive(Debug)]
    pub enum Type {
        End = 0,
        ForIdx = 1,
        ForStop = 2,
        ForStep = 3,
        ForGen = 4,
        ForState = 5,
        ForCtl = 6,
        String = 7,
    }

    impl Into<u8> for Type {
        fn into(self) -> u8 {
            return self as u8;
        }
    }

    impl From<u8> for Type {
        fn from(value: u8) -> Self {
            match value {
                0 => Type::End,
                1 => Type::ForIdx,
                2 => Type::ForStop,
                3 => Type::ForStep,
                4 => Type::ForGen,
                5 => Type::ForState,
                6 => Type::ForCtl,
                _ => Type::String,
            }
        }
    }

    pub struct Variable {
        pub name: String,
        pub tp: Type,
        pub scope: Range<u32>,
    }

    impl Variable {
        pub fn new<R>(data: &mut R, tp: u8) -> Self
        where
            R: Buf,
        {
            let name: String = if tp >= Type::String as u8 {
                let mut name = read_cstring(data).unwrap();
                name.insert(0, tp as char);
                name
            } else {
                "".to_string()
            };

            // TODO: The scope should be relative to the last variable's scope
            let scope = if tp != Type::End as u8 {
                Range {
                    start: data.read_leb(),
                    end: data.read_leb(),
                }
            } else {
                Range { start: 0, end: 0 }
            };

            Self {
                name: name,
                tp: match tp {
                    0 => Type::End,
                    1 => Type::ForIdx,
                    2 => Type::ForStop,
                    3 => Type::ForStep,
                    4 => Type::ForGen,
                    5 => Type::ForState,
                    6 => Type::ForCtl,
                    _ => Type::String,
                },
                scope,
            }
        }
    }

    impl fmt::Debug for Variable {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "Variable {{ type: {:#?}, name: {:#?}, scope: {:#?} }}",
                &self.tp, &self.name, &self.scope
            )
        }
    }
}

pub struct Debug {
    lines: Vec<i32>,
    upvalues: Vec<String>,
    variables: Vec<variable::Variable>,
}

impl Debug {
    pub fn new<R>(data: &mut impl EndianBuffer<R>, sizeinsn: usize, line_count: usize, upvalue_count: usize) -> Debug
    where
        R: Buf,
    {
        let mut lines = vec![0; sizeinsn];
        match line_count {
            65536.. => {
                (0..sizeinsn).for_each(|i| lines.insert(i, data.read_u32::<R>() as u32));
            }
            256.. => {
                (0..sizeinsn).for_each(|i| lines.insert(i, data.read_u16::<R>() as u32));
            }
            _ => {
                (0..sizeinsn).for_each(|i| lines.insert(i, data.get_u8() as u32));
            }
        };

        let mut upvalues = Vec::with_capacity(upvalue_count);
        for _ in 0..upvalue_count {
            match read_cstring(data.deref_mut()) {
                Some(str) => upvalues.push(str),
                None => panic!("Unable to parse string"),
            };
        }

        let mut vars = Vec::new();
        loop {
            let tp = data.get_u8();
            if tp == variable::Type::End.into() {
                break;
            }

            let var_info = variable::Variable::new(data.deref_mut(), tp);
            vars.push(var_info);
        }

        Self {
            lines: vec![],
            upvalues: upvalues,
            variables: vars,
        }
    }
}

impl fmt::Debug for Debug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Debug")
            .field("lines", &self.lines)
            .field("upvalues", &self.upvalues)
            .field("variables", &self.variables)
            .finish()
    }
}
