use core::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Colour {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl FromStr for Colour {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn get_single(iter: &mut impl Iterator<Item = char>) -> Result<u8, ()> {
            iter.next()
                .ok_or(())?
                .to_digit(16)
                .ok_or(())?
                .try_into()
                .or(Err(()))
        }
        fn get_double(iter: &mut impl Iterator<Item = char>) -> Result<u8, ()> {
            let upper = get_single(iter)?;
            let lower = get_single(iter)?;

            Ok(upper << 4 | lower)
        }

        match s.len() {
            1 => {
                let mut chars = s.chars();
                let value = get_single(&mut chars)?;
                Ok(Colour::new(value, value, value))
            }
            3 => {
                let mut chars = s.chars();
                let r = get_single(&mut chars)?;
                let g = get_single(&mut chars)?;
                let b = get_single(&mut chars)?;
                Ok(Colour::new(r, g, b))
            }
            4 => {
                let mut chars = s.chars();
                chars.next(); // # symbol
                let r = get_single(&mut chars)?;
                let g = get_single(&mut chars)?;
                let b = get_single(&mut chars)?;
                Ok(Colour::new(r, g, b))
            }
            6 => {
                let mut chars = s.chars();
                let r = get_double(&mut chars)?;
                let g = get_double(&mut chars)?;
                let b = get_double(&mut chars)?;
                Ok(Colour::new(r, g, b))
            }
            7 => {
                let mut chars = s.chars();
                chars.next(); // # symbol
                let r = get_double(&mut chars)?;
                let g = get_double(&mut chars)?;
                let b = get_double(&mut chars)?;
                Ok(Colour::new(r, g, b))
            }
            _ => Err(()),
        }
    }
}

#[allow(dead_code)]
pub static BLACK: Colour = Colour::new(0x00, 0x00, 0x00);
#[allow(dead_code)]
pub static WHITE: Colour = Colour::new(0xFF, 0xFF, 0xFF);
#[allow(dead_code)]
pub static RED: Colour = Colour::new(0xFF, 0x00, 0x00);
#[allow(dead_code)]
pub static GREEN: Colour = Colour::new(0x00, 0xFF, 0x00);
#[allow(dead_code)]
pub static BLUE: Colour = Colour::new(0x00, 0x00, 0xFF);
