use std::io;

pub trait DisplayPort {
    type Connection;

    fn connect(&self) -> io::Result<Self::Connection>;
}
