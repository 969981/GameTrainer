//! GDB Remote Serial Protocol for local mGBA. Writes GBA guest memory, not a Win32 process.
//! All read/write commands require confirmed stopped CPU. No raw commands exposed to React.
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    time::Duration,
};
use thiserror::Error;
const TIMEOUT: Duration = Duration::from_secs(4);
const MAX_PACKET: usize = 8192;
#[derive(Debug, Error)]
pub enum RspError {
    #[error("socket: {0}")]
    Io(#[from] std::io::Error),
    #[error("GDB protocol error: {0}")]
    Protocol(&'static str),
    #[error("target rejected: {0}")]
    Rejected(String),
    #[error("guest CPU not stopped")]
    Running,
    #[error("request outside limits")]
    Bounds,
}
pub struct RspClient {
    stream: TcpStream,
    paused: bool,
}
fn decode(bytes: &[u8]) -> Result<Vec<u8>, RspError> {
    if bytes.len() % 2 != 0 {
        return Err(RspError::Protocol("odd hex"));
    }
    bytes
        .chunks_exact(2)
        .map(|two| {
            let s = std::str::from_utf8(two).map_err(|_| RspError::Protocol("nonascii hex"))?;
            u8::from_str_radix(s, 16).map_err(|_| RspError::Protocol("bad hex"))
        })
        .collect()
}
fn encode(bytes: &[u8]) -> String {
    let mut s = String::new();
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
impl RspClient {
    pub fn connect_local(port: u16) -> Result<Self, RspError> {
        if port < 1024 {
            return Err(RspError::Bounds);
        }
        let addr = std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port));
        let stream = TcpStream::connect_timeout(&addr, TIMEOUT)?;
        stream.set_read_timeout(Some(TIMEOUT))?;
        stream.set_write_timeout(Some(TIMEOUT))?;
        stream.set_nodelay(true)?;
        let mut client = Self {
            stream,
            paused: false,
        };
        client.confirm_paused()?;
        Ok(client)
    }
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    fn send(&mut self, data: &[u8]) -> Result<(), RspError> {
        if data.len() + 4 > MAX_PACKET || data.contains(&b'    }
    fn receive(&mut self) -> Result<Vec<u8>, RspError> {
        let mut b = [0u8; 1];
        let mut found = false;
        for _ in 0..64 {
            self.stream.read_exact(&mut b)?;
            if b[0] == b'        self.stream.write_all(b"+")?;
        if data.starts_with(b"E") {
            return Err(RspError::Rejected(
                String::from_utf8_lossy(&data).into_owned(),
            ));
        }
        Ok(data)
    }
    fn command(&mut self, data: &[u8]) -> Result<Vec<u8>, RspError> {
        self.send(data)?;
        self.receive()
    }
    pub fn confirm_paused(&mut self) -> Result<(), RspError> {
        let reply = self.command(b"?")?;
        if (reply.starts_with(b"S") || reply.starts_with(b"T")) && reply.len() >= 3 {
            self.paused = true;
            self.pc()?;
            Ok(())
        } else {
            self.paused = false;
            Err(RspError::Running)
        }
    }
    pub fn pc(&mut self) -> Result<u32, RspError> {
        if !self.paused {
            return Err(RspError::Running);
        }
        let regs = self.command(b"g")?;
        if regs.len() < 128 {
            return Err(RspError::Protocol("short regs"));
        }
        let bytes: [u8; 4] = decode(&regs[120..128])?
            .try_into()
            .map_err(|_| RspError::Protocol("bad PC"))?;
        Ok(u32::from_le_bytes(bytes))
    }
    pub fn read(&mut self, addr: u32, length: usize) -> Result<Vec<u8>, RspError> {
        if !self.paused {
            return Err(RspError::Running);
        }
        if length > 16 * 1024 * 1024 || (addr as u64 + length as u64) > u32::MAX as u64 {
            return Err(RspError::Bounds);
        }
        let mut result = Vec::with_capacity(length);
        while result.len() < length {
            let size = (length - result.len()).min(256);
            let at = addr + result.len() as u32;
            let bytes = self.command(format!("m{at:x},{size:x}").as_bytes())?;
            if bytes.len() != size * 2 {
                return Err(RspError::Protocol("short read"));
            }
            result.extend_from_slice(&decode(&bytes)?);
        }
        Ok(result)
    }
    pub fn write(&mut self, addr: u32, data: &[u8]) -> Result<(), RspError> {
        if !self.paused {
            return Err(RspError::Running);
        }
        if data.is_empty()
            || data.len() > 512
            || (addr as u64 + data.len() as u64) > u32::MAX as u64
        {
            return Err(RspError::Bounds);
        }
        let cmd = format!("M{addr:x},{:x}:{}", data.len(), encode(data));
        if self.command(cmd.as_bytes())? != b"OK" {
            return Err(RspError::Protocol("write not OK"));
        }
        if self.read(addr, data.len())? != data {
            return Err(RspError::Protocol("write readback mismatch"));
        }
        Ok(())
    }
    pub fn resume(&mut self) -> Result<(), RspError> {
        if !self.paused {
            return Err(RspError::Running);
        }
        self.send(b"c")?;
        let mut ack = [0u8; 1];
        self.stream.read_exact(&mut ack)?;
        if ack[0] != b'+' {
            return Err(RspError::Protocol("resume not acknowledged"));
        }
        self.paused = false;
        Ok(())
    }
    pub fn interrupt(&mut self) -> Result<(), RspError> {
        if self.paused {
            return self.confirm_paused();
        }
        self.stream.write_all(&[0x03])?;
        let reply = self.receive()?;
        if (reply.starts_with(b"S") || reply.starts_with(b"T")) && reply.len() >= 3 {
            self.paused = true;
            self.pc()?;
            Ok(())
        } else {
            Err(RspError::Running)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hex_helpers() {
        assert_eq!(decode(b"00aaff").unwrap(), [0, 170, 255]);
        assert_eq!(encode(&[0, 170, 255]), "00aaff");
        assert!(decode(b"zz").is_err());
    }
    #[test]
    fn refuse_unprivileged_ports() {
        assert!(matches!(
            RspClient::connect_local(23),
            Err(RspError::Bounds)
        ));
    }
}
) || data.contains(&b'#') {
            return Err(RspError::Bounds);
        }
        let sum = data.iter().fold(0u8, |a, b| a.wrapping_add(*b));
        let mut packet = Vec::with_capacity(data.len() + 4);
        packet.push(b'    }
    fn receive(&mut self)->Result<Vec<u8>,RspError>{
        let mut b=[0u8;1];
        let mut found=false;
        for _ in 0..64{
            self.stream.read_exact(&mut b)?;
            if b[0]==b'$'{found=true;break;}
            if b[0]!=b'+'{return Err(RspError::Protocol("bad ack"));}
        }
        if !found{return Err(RspError::Bounds);}
        let mut data=Vec::new();
        loop{
            self.stream.read_exact(&mut b)?;
            if b[0]==b'#'{break;}
            if data.len()>=MAX_PACKET{return Err(RspError::Bounds);}
            data.push(b[0]);
        }
        let mut check=[0u8;2];
        self.stream.read_exact(&mut check)?;
        let expected=u8::from_str_radix(std::str::from_utf8(&check)
            .map_err(|_|RspError::Protocol("checksum"))?,16)
            .map_err(|_|RspError::Protocol("checksum"))?;
        if expected!=data.iter().fold(0u8,|a,b|a.wrapping_add(*b)){
            self.stream.write_all(b"-")?;
            return Err(RspError::Protocol("checksum mismatch"));
        }
        self.stream.write_all(b"+")?;
        if data.starts_with(b"E"){return Err(RspError::Rejected(String::from_utf8_lossy(&data).into_owned()));}
        Ok(data)
    }
    fn command(&mut self,data:&[u8])->Result<Vec<u8>,RspError>{
        self.send(data)?;
        self.receive()
    }
    pub fn confirm_paused(&mut self)->Result<(),RspError>{
        let reply=self.command(b"?")?;
        if (reply.starts_with(b"S")||reply.starts_with(b"T"))&&reply.len()>=3{
            self.paused=true;self.pc()?;Ok(())
        }else{self.paused=false;Err(RspError::Running)}
    }
    pub fn pc(&mut self)->Result<u32,RspError>{
        if !self.paused{return Err(RspError::Running);}
        let regs=self.command(b"g")?;
        if regs.len()<128{return Err(RspError::Protocol("short regs"));}
        let bytes:[u8;4]=decode(&regs[120..128])?.try_into().map_err(|_|RspError::Protocol("bad PC"))?;
        Ok(u32::from_le_bytes(bytes))
    }
    pub fn read(&mut self,addr:u32,length:usize)->Result<Vec<u8>,RspError>{
        if !self.paused{return Err(RspError::Running);}
        if length>16*1024*1024||(addr as u64+length as u64)>u32::MAX as u64{return Err(RspError::Bounds);}
        let mut result=Vec::with_capacity(length);
        while result.len()<length{
            let size=(length-result.len()).min(256);
            let at=addr+result.len() as u32;
            let bytes=self.command(format!("m{at:x},{size:x}").as_bytes())?;
            if bytes.len()!=size*2{return Err(RspError::Protocol("short read"));}
            result.extend_from_slice(&decode(&bytes)?);
        }
        Ok(result)
    }
    pub fn write(&mut self,addr:u32,data:&[u8])->Result<(),RspError>{
        if !self.paused{return Err(RspError::Running);}
        if data.is_empty()||data.len()>512||(addr as u64+data.len() as u64)>u32::MAX as u64{return Err(RspError::Bounds);}
        let cmd=format!("M{addr:x},{:x}:{}",data.len(),encode(data));
        if self.command(cmd.as_bytes())?!=b"OK"{return Err(RspError::Protocol("write not OK"));}
        if self.read(addr,data.len())?!=data{return Err(RspError::Protocol("write readback mismatch"));}
        Ok(())
    }
    pub fn resume(&mut self)->Result<(),RspError>{
        if !self.paused{return Err(RspError::Running);}
        self.send(b"c")?;
        let mut ack=[0u8;1];self.stream.read_exact(&mut ack)?;
        if ack[0]!=b'+'{return Err(RspError::Protocol("resume not acknowledged"));}
        self.paused=false;Ok(())
    }
    pub fn interrupt(&mut self)->Result<(),RspError>{
        if self.paused{return self.confirm_paused();}
        self.stream.write_all(&[0x03])?;
        let reply=self.receive()?;
        if (reply.starts_with(b"S")||reply.starts_with(b"T"))&&reply.len()>=3{
            self.paused=true;self.pc()?;Ok(())
        }else{Err(RspError::Running)}
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn hex_helpers(){assert_eq!(decode(b"00aaff").unwrap(),[0,170,255]);assert_eq!(encode(&[0,170,255]),"00aaff");assert!(decode(b"zz").is_err());}
    #[test]
    fn refuse_unprivileged_ports(){assert!(matches!(RspClient::connect_local(23),Err(RspError::Bounds)));}
}
);
        packet.extend_from_slice(data);
        packet.extend_from_slice(format!("#{sum:02x}").as_bytes());
        self.stream.write_all(&packet)?;
        Ok(())
    }
    fn receive(&mut self)->Result<Vec<u8>,RspError>{
        let mut b=[0u8;1];
        let mut found=false;
        for _ in 0..64{
            self.stream.read_exact(&mut b)?;
            if b[0]==b'$'{found=true;break;}
            if b[0]!=b'+'{return Err(RspError::Protocol("bad ack"));}
        }
        if !found{return Err(RspError::Bounds);}
        let mut data=Vec::new();
        loop{
            self.stream.read_exact(&mut b)?;
            if b[0]==b'#'{break;}
            if data.len()>=MAX_PACKET{return Err(RspError::Bounds);}
            data.push(b[0]);
        }
        let mut check=[0u8;2];
        self.stream.read_exact(&mut check)?;
        let expected=u8::from_str_radix(std::str::from_utf8(&check)
            .map_err(|_|RspError::Protocol("checksum"))?,16)
            .map_err(|_|RspError::Protocol("checksum"))?;
        if expected!=data.iter().fold(0u8,|a,b|a.wrapping_add(*b)){
            self.stream.write_all(b"-")?;
            return Err(RspError::Protocol("checksum mismatch"));
        }
        self.stream.write_all(b"+")?;
        if data.starts_with(b"E"){return Err(RspError::Rejected(String::from_utf8_lossy(&data).into_owned()));}
        Ok(data)
    }
    fn command(&mut self,data:&[u8])->Result<Vec<u8>,RspError>{
        self.send(data)?;
        self.receive()
    }
    pub fn confirm_paused(&mut self)->Result<(),RspError>{
        let reply=self.command(b"?")?;
        if (reply.starts_with(b"S")||reply.starts_with(b"T"))&&reply.len()>=3{
            self.paused=true;self.pc()?;Ok(())
        }else{self.paused=false;Err(RspError::Running)}
    }
    pub fn pc(&mut self)->Result<u32,RspError>{
        if !self.paused{return Err(RspError::Running);}
        let regs=self.command(b"g")?;
        if regs.len()<128{return Err(RspError::Protocol("short regs"));}
        let bytes:[u8;4]=decode(&regs[120..128])?.try_into().map_err(|_|RspError::Protocol("bad PC"))?;
        Ok(u32::from_le_bytes(bytes))
    }
    pub fn read(&mut self,addr:u32,length:usize)->Result<Vec<u8>,RspError>{
        if !self.paused{return Err(RspError::Running);}
        if length>16*1024*1024||(addr as u64+length as u64)>u32::MAX as u64{return Err(RspError::Bounds);}
        let mut result=Vec::with_capacity(length);
        while result.len()<length{
            let size=(length-result.len()).min(256);
            let at=addr+result.len() as u32;
            let bytes=self.command(format!("m{at:x},{size:x}").as_bytes())?;
            if bytes.len()!=size*2{return Err(RspError::Protocol("short read"));}
            result.extend_from_slice(&decode(&bytes)?);
        }
        Ok(result)
    }
    pub fn write(&mut self,addr:u32,data:&[u8])->Result<(),RspError>{
        if !self.paused{return Err(RspError::Running);}
        if data.is_empty()||data.len()>512||(addr as u64+data.len() as u64)>u32::MAX as u64{return Err(RspError::Bounds);}
        let cmd=format!("M{addr:x},{:x}:{}",data.len(),encode(data));
        if self.command(cmd.as_bytes())?!=b"OK"{return Err(RspError::Protocol("write not OK"));}
        if self.read(addr,data.len())?!=data{return Err(RspError::Protocol("write readback mismatch"));}
        Ok(())
    }
    pub fn resume(&mut self)->Result<(),RspError>{
        if !self.paused{return Err(RspError::Running);}
        self.send(b"c")?;
        let mut ack=[0u8;1];self.stream.read_exact(&mut ack)?;
        if ack[0]!=b'+'{return Err(RspError::Protocol("resume not acknowledged"));}
        self.paused=false;Ok(())
    }
    pub fn interrupt(&mut self)->Result<(),RspError>{
        if self.paused{return self.confirm_paused();}
        self.stream.write_all(&[0x03])?;
        let reply=self.receive()?;
        if (reply.starts_with(b"S")||reply.starts_with(b"T"))&&reply.len()>=3{
            self.paused=true;self.pc()?;Ok(())
        }else{Err(RspError::Running)}
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn hex_helpers(){assert_eq!(decode(b"00aaff").unwrap(),[0,170,255]);assert_eq!(encode(&[0,170,255]),"00aaff");assert!(decode(b"zz").is_err());}
    #[test]
    fn refuse_unprivileged_ports(){assert!(matches!(RspClient::connect_local(23),Err(RspError::Bounds)));}
}
 {
                found = true;
                break;
            }
            if b[0] != b'+' {
                return Err(RspError::Protocol("bad ack"));
            }
        }
        if !found {
            return Err(RspError::Bounds);
        }
        let mut data = Vec::new();
        loop {
            self.stream.read_exact(&mut b)?;
            if b[0] == b'#' {
                break;
            }
            if data.len() >= MAX_PACKET {
                return Err(RspError::Bounds);
            }
            data.push(b[0]);
        }
        let mut check = [0u8; 2];
        self.stream.read_exact(&mut check)?;
        let expected = u8::from_str_radix(
            std::str::from_utf8(&check).map_err(|_| RspError::Protocol("checksum"))?,
            16,
        )
        .map_err(|_| RspError::Protocol("checksum"))?;
        if expected != data.iter().fold(0u8, |a, b| a.wrapping_add(*b)) {
            self.stream.write_all(b"-")?;
            return Err(RspError::Protocol("checksum mismatch"));
        }
        self.stream.write_all(b"+")?;
        if data.starts_with(b"E"){return Err(RspError::Rejected(String::from_utf8_lossy(&data).into_owned()));}
        Ok(data)
    }
    fn command(&mut self,data:&[u8])->Result<Vec<u8>,RspError>{
        self.send(data)?;
        self.receive()
    }
    pub fn confirm_paused(&mut self)->Result<(),RspError>{
        let reply=self.command(b"?")?;
        if (reply.starts_with(b"S")||reply.starts_with(b"T"))&&reply.len()>=3{
            self.paused=true;self.pc()?;Ok(())
        }else{self.paused=false;Err(RspError::Running)}
    }
    pub fn pc(&mut self)->Result<u32,RspError>{
        if !self.paused{return Err(RspError::Running);}
        let regs=self.command(b"g")?;
        if regs.len()<128{return Err(RspError::Protocol("short regs"));}
        let bytes:[u8;4]=decode(&regs[120..128])?.try_into().map_err(|_|RspError::Protocol("bad PC"))?;
        Ok(u32::from_le_bytes(bytes))
    }
    pub fn read(&mut self,addr:u32,length:usize)->Result<Vec<u8>,RspError>{
        if !self.paused{return Err(RspError::Running);}
        if length>16*1024*1024||(addr as u64+length as u64)>u32::MAX as u64{return Err(RspError::Bounds);}
        let mut result=Vec::with_capacity(length);
        while result.len()<length{
            let size=(length-result.len()).min(256);
            let at=addr+result.len() as u32;
            let bytes=self.command(format!("m{at:x},{size:x}").as_bytes())?;
            if bytes.len()!=size*2{return Err(RspError::Protocol("short read"));}
            result.extend_from_slice(&decode(&bytes)?);
        }
        Ok(result)
    }
    pub fn write(&mut self,addr:u32,data:&[u8])->Result<(),RspError>{
        if !self.paused{return Err(RspError::Running);}
        if data.is_empty()||data.len()>512||(addr as u64+data.len() as u64)>u32::MAX as u64{return Err(RspError::Bounds);}
        let cmd=format!("M{addr:x},{:x}:{}",data.len(),encode(data));
        if self.command(cmd.as_bytes())?!=b"OK"{return Err(RspError::Protocol("write not OK"));}
        if self.read(addr,data.len())?!=data{return Err(RspError::Protocol("write readback mismatch"));}
        Ok(())
    }
    pub fn resume(&mut self)->Result<(),RspError>{
        if !self.paused{return Err(RspError::Running);}
        self.send(b"c")?;
        let mut ack=[0u8;1];self.stream.read_exact(&mut ack)?;
        if ack[0]!=b'+'{return Err(RspError::Protocol("resume not acknowledged"));}
        self.paused=false;Ok(())
    }
    pub fn interrupt(&mut self)->Result<(),RspError>{
        if self.paused{return self.confirm_paused();}
        self.stream.write_all(&[0x03])?;
        let reply=self.receive()?;
        if (reply.starts_with(b"S")||reply.starts_with(b"T"))&&reply.len()>=3{
            self.paused=true;self.pc()?;Ok(())
        }else{Err(RspError::Running)}
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn hex_helpers(){assert_eq!(decode(b"00aaff").unwrap(),[0,170,255]);assert_eq!(encode(&[0,170,255]),"00aaff");assert!(decode(b"zz").is_err());}
    #[test]
    fn refuse_unprivileged_ports(){assert!(matches!(RspClient::connect_local(23),Err(RspError::Bounds)));}
}
) || data.contains(&b'#') {
            return Err(RspError::Bounds);
        }
        let sum = data.iter().fold(0u8, |a, b| a.wrapping_add(*b));
        let mut packet = Vec::with_capacity(data.len() + 4);
        packet.push(b'    }
    fn receive(&mut self)->Result<Vec<u8>,RspError>{
        let mut b=[0u8;1];
        let mut found=false;
        for _ in 0..64{
            self.stream.read_exact(&mut b)?;
            if b[0]==b'$'{found=true;break;}
            if b[0]!=b'+'{return Err(RspError::Protocol("bad ack"));}
        }
        if !found{return Err(RspError::Bounds);}
        let mut data=Vec::new();
        loop{
            self.stream.read_exact(&mut b)?;
            if b[0]==b'#'{break;}
            if data.len()>=MAX_PACKET{return Err(RspError::Bounds);}
            data.push(b[0]);
        }
        let mut check=[0u8;2];
        self.stream.read_exact(&mut check)?;
        let expected=u8::from_str_radix(std::str::from_utf8(&check)
            .map_err(|_|RspError::Protocol("checksum"))?,16)
            .map_err(|_|RspError::Protocol("checksum"))?;
        if expected!=data.iter().fold(0u8,|a,b|a.wrapping_add(*b)){
            self.stream.write_all(b"-")?;
            return Err(RspError::Protocol("checksum mismatch"));
        }
        self.stream.write_all(b"+")?;
        if data.starts_with(b"E"){return Err(RspError::Rejected(String::from_utf8_lossy(&data).into_owned()));}
        Ok(data)
    }
    fn command(&mut self,data:&[u8])->Result<Vec<u8>,RspError>{
        self.send(data)?;
        self.receive()
    }
    pub fn confirm_paused(&mut self)->Result<(),RspError>{
        let reply=self.command(b"?")?;
        if (reply.starts_with(b"S")||reply.starts_with(b"T"))&&reply.len()>=3{
            self.paused=true;self.pc()?;Ok(())
        }else{self.paused=false;Err(RspError::Running)}
    }
    pub fn pc(&mut self)->Result<u32,RspError>{
        if !self.paused{return Err(RspError::Running);}
        let regs=self.command(b"g")?;
        if regs.len()<128{return Err(RspError::Protocol("short regs"));}
        let bytes:[u8;4]=decode(&regs[120..128])?.try_into().map_err(|_|RspError::Protocol("bad PC"))?;
        Ok(u32::from_le_bytes(bytes))
    }
    pub fn read(&mut self,addr:u32,length:usize)->Result<Vec<u8>,RspError>{
        if !self.paused{return Err(RspError::Running);}
        if length>16*1024*1024||(addr as u64+length as u64)>u32::MAX as u64{return Err(RspError::Bounds);}
        let mut result=Vec::with_capacity(length);
        while result.len()<length{
            let size=(length-result.len()).min(256);
            let at=addr+result.len() as u32;
            let bytes=self.command(format!("m{at:x},{size:x}").as_bytes())?;
            if bytes.len()!=size*2{return Err(RspError::Protocol("short read"));}
            result.extend_from_slice(&decode(&bytes)?);
        }
        Ok(result)
    }
    pub fn write(&mut self,addr:u32,data:&[u8])->Result<(),RspError>{
        if !self.paused{return Err(RspError::Running);}
        if data.is_empty()||data.len()>512||(addr as u64+data.len() as u64)>u32::MAX as u64{return Err(RspError::Bounds);}
        let cmd=format!("M{addr:x},{:x}:{}",data.len(),encode(data));
        if self.command(cmd.as_bytes())?!=b"OK"{return Err(RspError::Protocol("write not OK"));}
        if self.read(addr,data.len())?!=data{return Err(RspError::Protocol("write readback mismatch"));}
        Ok(())
    }
    pub fn resume(&mut self)->Result<(),RspError>{
        if !self.paused{return Err(RspError::Running);}
        self.send(b"c")?;
        let mut ack=[0u8;1];self.stream.read_exact(&mut ack)?;
        if ack[0]!=b'+'{return Err(RspError::Protocol("resume not acknowledged"));}
        self.paused=false;Ok(())
    }
    pub fn interrupt(&mut self)->Result<(),RspError>{
        if self.paused{return self.confirm_paused();}
        self.stream.write_all(&[0x03])?;
        let reply=self.receive()?;
        if (reply.starts_with(b"S")||reply.starts_with(b"T"))&&reply.len()>=3{
            self.paused=true;self.pc()?;Ok(())
        }else{Err(RspError::Running)}
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn hex_helpers(){assert_eq!(decode(b"00aaff").unwrap(),[0,170,255]);assert_eq!(encode(&[0,170,255]),"00aaff");assert!(decode(b"zz").is_err());}
    #[test]
    fn refuse_unprivileged_ports(){assert!(matches!(RspClient::connect_local(23),Err(RspError::Bounds)));}
}
);
        packet.extend_from_slice(data);
        packet.extend_from_slice(format!("#{sum:02x}").as_bytes());
        self.stream.write_all(&packet)?;
        Ok(())
    }
    fn receive(&mut self)->Result<Vec<u8>,RspError>{
        let mut b=[0u8;1];
        let mut found=false;
        for _ in 0..64{
            self.stream.read_exact(&mut b)?;
            if b[0]==b'$'{found=true;break;}
            if b[0]!=b'+'{return Err(RspError::Protocol("bad ack"));}
        }
        if !found{return Err(RspError::Bounds);}
        let mut data=Vec::new();
        loop{
            self.stream.read_exact(&mut b)?;
            if b[0]==b'#'{break;}
            if data.len()>=MAX_PACKET{return Err(RspError::Bounds);}
            data.push(b[0]);
        }
        let mut check=[0u8;2];
        self.stream.read_exact(&mut check)?;
        let expected=u8::from_str_radix(std::str::from_utf8(&check)
            .map_err(|_|RspError::Protocol("checksum"))?,16)
            .map_err(|_|RspError::Protocol("checksum"))?;
        if expected!=data.iter().fold(0u8,|a,b|a.wrapping_add(*b)){
            self.stream.write_all(b"-")?;
            return Err(RspError::Protocol("checksum mismatch"));
        }
        self.stream.write_all(b"+")?;
        if data.starts_with(b"E"){return Err(RspError::Rejected(String::from_utf8_lossy(&data).into_owned()));}
        Ok(data)
    }
    fn command(&mut self,data:&[u8])->Result<Vec<u8>,RspError>{
        self.send(data)?;
        self.receive()
    }
    pub fn confirm_paused(&mut self)->Result<(),RspError>{
        let reply=self.command(b"?")?;
        if (reply.starts_with(b"S")||reply.starts_with(b"T"))&&reply.len()>=3{
            self.paused=true;self.pc()?;Ok(())
        }else{self.paused=false;Err(RspError::Running)}
    }
    pub fn pc(&mut self)->Result<u32,RspError>{
        if !self.paused{return Err(RspError::Running);}
        let regs=self.command(b"g")?;
        if regs.len()<128{return Err(RspError::Protocol("short regs"));}
        let bytes:[u8;4]=decode(&regs[120..128])?.try_into().map_err(|_|RspError::Protocol("bad PC"))?;
        Ok(u32::from_le_bytes(bytes))
    }
    pub fn read(&mut self,addr:u32,length:usize)->Result<Vec<u8>,RspError>{
        if !self.paused{return Err(RspError::Running);}
        if length>16*1024*1024||(addr as u64+length as u64)>u32::MAX as u64{return Err(RspError::Bounds);}
        let mut result=Vec::with_capacity(length);
        while result.len()<length{
            let size=(length-result.len()).min(256);
            let at=addr+result.len() as u32;
            let bytes=self.command(format!("m{at:x},{size:x}").as_bytes())?;
            if bytes.len()!=size*2{return Err(RspError::Protocol("short read"));}
            result.extend_from_slice(&decode(&bytes)?);
        }
        Ok(result)
    }
    pub fn write(&mut self,addr:u32,data:&[u8])->Result<(),RspError>{
        if !self.paused{return Err(RspError::Running);}
        if data.is_empty()||data.len()>512||(addr as u64+data.len() as u64)>u32::MAX as u64{return Err(RspError::Bounds);}
        let cmd=format!("M{addr:x},{:x}:{}",data.len(),encode(data));
        if self.command(cmd.as_bytes())?!=b"OK"{return Err(RspError::Protocol("write not OK"));}
        if self.read(addr,data.len())?!=data{return Err(RspError::Protocol("write readback mismatch"));}
        Ok(())
    }
    pub fn resume(&mut self)->Result<(),RspError>{
        if !self.paused{return Err(RspError::Running);}
        self.send(b"c")?;
        let mut ack=[0u8;1];self.stream.read_exact(&mut ack)?;
        if ack[0]!=b'+'{return Err(RspError::Protocol("resume not acknowledged"));}
        self.paused=false;Ok(())
    }
    pub fn interrupt(&mut self)->Result<(),RspError>{
        if self.paused{return self.confirm_paused();}
        self.stream.write_all(&[0x03])?;
        let reply=self.receive()?;
        if (reply.starts_with(b"S")||reply.starts_with(b"T"))&&reply.len()>=3{
            self.paused=true;self.pc()?;Ok(())
        }else{Err(RspError::Running)}
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn hex_helpers(){assert_eq!(decode(b"00aaff").unwrap(),[0,170,255]);assert_eq!(encode(&[0,170,255]),"00aaff");assert!(decode(b"zz").is_err());}
    #[test]
    fn refuse_unprivileged_ports(){assert!(matches!(RspClient::connect_local(23),Err(RspError::Bounds)));}
}
