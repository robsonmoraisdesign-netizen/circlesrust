// Image output for Rust diagnostic/path-tracing previews.
// PNG is dependency-free using zlib stored DEFLATE blocks; BMP compatibility
// is retained from the first CPU/BVH preview.
pub mod denoise;

use std::io;
use std::path::Path;

fn validate_rgb(rgb:&[u8],width:u32,height:u32)->io::Result<()>{
    let required=(width as usize).checked_mul(height as usize)
        .and_then(|n|n.checked_mul(3))
        .ok_or_else(||io::Error::new(io::ErrorKind::InvalidInput,"image too large"))?;
    if width==0 || height==0 || rgb.len()!=required{
        Err(io::Error::new(io::ErrorKind::InvalidInput,"invalid RGB dimensions"))
    }else{Ok(())}
}

pub fn bmp_bytes(rgb:&[u8],width:u32,height:u32)->io::Result<Vec<u8>>{
    validate_rgb(rgb,width,height)?;
    let stride=((width as usize)*3+3)&!3;
    let size=54+stride*height as usize;
    let size_u32=u32::try_from(size).map_err(|_|io::Error::new(io::ErrorKind::InvalidInput,"BMP > 4 GiB"))?;
    let mut bytes=vec![0u8;size];
    bytes[0..2].copy_from_slice(b"BM");
    bytes[2..6].copy_from_slice(&size_u32.to_le_bytes());
    bytes[10..14].copy_from_slice(&54u32.to_le_bytes());
    bytes[14..18].copy_from_slice(&40u32.to_le_bytes());
    bytes[18..22].copy_from_slice(&width.to_le_bytes());
    bytes[22..26].copy_from_slice(&height.to_le_bytes());
    bytes[26..28].copy_from_slice(&1u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&24u16.to_le_bytes());
    bytes[34..38].copy_from_slice(&((stride*height as usize) as u32).to_le_bytes());
    for y in 0..height as usize{
        let src=y*width as usize*3;
        let dest=54+(height as usize-1-y)*stride;
        for x in 0..width as usize{
            let a=src+x*3;let b=dest+x*3;
            bytes[b..b+3].copy_from_slice(&[rgb[a+2],rgb[a+1],rgb[a]]);
        }
    }
    Ok(bytes)
}

fn crc32(data:&[u8])->u32{
    let mut crc=!0u32;
    for &byte in data{
        crc^=byte as u32;
        for _ in 0..8{
            crc=if crc&1==1{(crc>>1)^0xedb8_8320}else{crc>>1};
        }
    }
    !crc
}
fn png_chunk(out:&mut Vec<u8>,tag:&[u8;4],payload:&[u8]){
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    let offset=out.len();
    out.extend_from_slice(tag);
    out.extend_from_slice(payload);
    out.extend_from_slice(&crc32(&out[offset..]).to_be_bytes());
}
fn adler32(data:&[u8])->u32{
    let(mut a,mut b)=(1u32,0u32);
    for &byte in data{
        a=(a+byte as u32)%65521;
        b=(b+a)%65521;
    }
    (b<<16)|a
}
pub fn png_bytes(rgb:&[u8],width:u32,height:u32)->io::Result<Vec<u8>>{
    validate_rgb(rgb,width,height)?;
    let mut raw=Vec::with_capacity(rgb.len()+height as usize);
    let row_size=width as usize*3;
    for y in 0..height as usize{
        raw.push(0); // PNG filter method "None".
        raw.extend_from_slice(&rgb[y*row_size..(y+1)*row_size]);
    }
    // A valid zlib stream of stored deflate blocks (fast, lossless).
    let mut compressed=Vec::with_capacity(raw.len()+raw.len()/65535*5+20);
    compressed.extend_from_slice(&[0x78,0x01]);
    let mut offset=0usize;
    while offset<raw.len(){
        let take=(raw.len()-offset).min(65535);
        compressed.push(if offset+take==raw.len(){1}else{0});
        let len=take as u16;
        compressed.extend_from_slice(&len.to_le_bytes());
        compressed.extend_from_slice(&(!len).to_le_bytes());
        compressed.extend_from_slice(&raw[offset..offset+take]);
        offset+=take;
    }
    compressed.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut out=Vec::with_capacity(compressed.len()+64);
    out.extend_from_slice(&[137,80,78,71,13,10,26,10]);
    let mut header=[0u8;13];
    header[..4].copy_from_slice(&width.to_be_bytes());
    header[4..8].copy_from_slice(&height.to_be_bytes());
    header[8]=8;  // 8-bit/channel
    header[9]=2;  // truecolor RGB
    png_chunk(&mut out,b"IHDR",&header);
    png_chunk(&mut out,b"IDAT",&compressed);
    png_chunk(&mut out,b"IEND",&[]);
    Ok(out)
}

pub fn save_image(path:impl AsRef<Path>,rgb:&[u8],width:u32,height:u32)->io::Result<()>{
    let path=path.as_ref();
    let extension=path.extension().and_then(|x|x.to_str()).unwrap_or("").to_ascii_lowercase();
    let bytes=match extension.as_str(){
        "png"=>png_bytes(rgb,width,height)?,
        "bmp"=>bmp_bytes(rgb,width,height)?,
        _=>return Err(io::Error::new(io::ErrorKind::InvalidInput,"output file extension must be PNG or BMP")),
    };
    std::fs::write(path,bytes)
}
