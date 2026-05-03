pub mod client;
pub mod server;

// pub fn parse_byte_array(input: &[u8]) -> IResult<&[u8], &[u8]> {
//     let (rest, raw) = take(1024usize).parse(input)?;
//     // Retirer le padding de null bytes à droite
//     let trimmed = raw
//         .iter()
//         .rposition(|&b| b != 0x00)
//         .map(|pos| &raw[..=pos])
//         .unwrap_or(&raw[..0]);
//     Ok((rest, trimmed))
// }
