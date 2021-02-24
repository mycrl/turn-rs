
trait Codec {
    type Base;
    type Error;
    fn decoder(buf: &[u8]) -> Result<Self::Base, Self::Error>;
    fn encoder(self, buf: &mut [u8]) -> Result<(), Self::Error>;
}

struct A(u32);
impl Codec for A {
    type Base = u32;
    type Error = anyhow::Error;

    fn decoder(buf: &[u8]) -> Result<Self::Base, Self::Error> {
        Ok(u32::from_be_bytes([
            buf[0],
            buf[1],
            buf[2],
            buf[3]
        ]))
    }

    fn encoder(self, buf: &mut [u8]) -> Result<(), Self::Error> {
        let source = self.0.to_be_bytes();
        buf[0] = source[0];
        buf[1] = source[1];
        buf[2] = source[2];
        buf[3] = source[3];
        Ok(())
    }
}

fn try_from<C: Codec>(buf: &[u8]) -> Result<C::Base, C::Error> {
    C::decoder(buf)
}

fn try_into<C: Codec>(codec: C, buf: &mut [u8]) -> Result<(), C::Error> {
    codec.encoder(buf)
}

fn main() {
    let buf = [0u8, 0, 0, 1];
    println!("{:?}", try_from::<A>(&buf).unwrap());
}
