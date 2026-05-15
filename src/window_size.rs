#[derive(Debug, PartialEq)]
pub enum WindowSize {
    Size128,
    Size256,
    Size512,
    Size1024,
    Size2048,
    Size4096,
    Size8192,
    Size16384,
    Size32768,
    Custom(usize),
}

impl From<usize> for WindowSize {
    fn from(item: usize) -> Self {
        match item {
            128 => Self::Size128,
            256 => Self::Size256,
            512 => Self::Size512,
            1024 => Self::Size1024,
            2048 => Self::Size2048,
            4096 => Self::Size4096,
            8192 => Self::Size8192,
            16384 => Self::Size16384,
            32768 => Self::Size32768,
            _ => Self::Custom(item),
        }
    }
}

impl WindowSize {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Size128 => "128",
            Self::Size256 => "256",
            Self::Size512 => "512",
            Self::Size1024 => "1024",
            Self::Size2048 => "2048",
            Self::Size4096 => "4096",
            Self::Size8192 => "8192",
            Self::Size16384 => "16384",
            Self::Size32768 => "32768",
            Self::Custom(_) => "",
        }
    }

    pub fn value(&self) -> usize {
        match self {
            Self::Size128 => 128,
            Self::Size256 => 256,
            Self::Size512 => 512,
            Self::Size1024 => 1024,
            Self::Size2048 => 2048,
            Self::Size4096 => 4096,
            Self::Size8192 => 8192,
            Self::Size16384 => 16384,
            Self::Size32768 => 32768,
            Self::Custom(x) => *x,
        }
    }

    pub fn into_iter() -> impl Iterator<Item = Self> {
        [
            Self::Size128,
            Self::Size256,
            Self::Size512,
            Self::Size1024,
            Self::Size2048,
            Self::Size4096,
            Self::Size8192,
            Self::Size16384,
            Self::Size32768,
        ]
        .into_iter()
    }
}
