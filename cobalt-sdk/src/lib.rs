use std::{
    fmt::Display,
    io::{self, Write},
};

use cobalt_protocol::{Decode, Encode, MAX_STRING_LENGTH, deserialize_string_with_max};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Peaceful = 0,
    Easy = 1,
    Normal = 2,
    Hard = 3,
}

impl Encode for Difficulty {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let value = *self as u8;
        u8::encode(&value, writer)
    }
}

impl Decode for Difficulty {
    fn decode<R: io::Read + Unpin>(reader: &mut R) -> io::Result<Self> {
        let _value = u8::decode(reader)?;
        todo!()
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gamemode {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}

#[derive(Debug)]
pub struct GamemodeState {
    pub gamemode: Gamemode,
    pub hardcore: bool,
}

impl GamemodeState {
    pub fn new(gamemode: Gamemode) -> Self {
        Self {
            gamemode,
            hardcore: false,
        }
    }
}

impl Encode for GamemodeState {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let value = self.gamemode as u8 | if self.hardcore { 0x8 } else { 0 };
        u8::encode(&value, writer)
    }
}

impl Decode for GamemodeState {
    fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    Overworld = 0,
    Nether = -1,
    End = 1,
}

impl Encode for Dimension {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let value = *self as i8;
        i8::encode(&value, writer)
    }
}

impl Decode for Dimension {
    fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WorldType {
    Default,
    Flat,
    LargeBiomes,
    Amplified,
}

impl Encode for WorldType {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let value = match self {
            Self::Default => "default",
            Self::Flat => "flat",
            Self::LargeBiomes => "largeBiomes",
            Self::Amplified => "amplified",
        };
        str::encode(value, writer)
    }
}

impl Decode for WorldType {
    fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
        todo!()
    }
}

#[derive(Debug)]
pub enum PluginChannel {
    McBrand,
}

impl Display for PluginChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::McBrand => write!(f, "MC|Brand"),
        }
    }
}

impl Encode for PluginChannel {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        str::encode(&self.to_string(), writer)
    }
}

impl Decode for PluginChannel {
    fn decode<R: io::Read + Unpin>(reader: &mut R) -> io::Result<Self> {
        let value = deserialize_string_with_max(reader, MAX_STRING_LENGTH)?;
        Self::try_from(value)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid adata"))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("invalid value")]
    InvalidValue,
}

impl TryFrom<String> for PluginChannel {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for PluginChannel {
    type Error = DecodeError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "MC|Brand" => Ok(Self::McBrand),
            _ => Err(Self::Error::InvalidValue),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
#[repr(i16)]
pub enum ItemId {
    Empty = -1,
    #[default]
    Air = 0,
    Stone = 1,
    Grass = 2,
    Dirt = 3,
    Cobblestone = 4,
    Planks = 5,
    Sapling = 6,
    Bedrock = 7,
    Water = 8,
    StationaryWater = 9,
    Lava = 10,
    StationaryLava = 11,
    Sand = 12,
    Gravel = 13,
}
//

impl From<ItemId> for i16 {
    fn from(id: ItemId) -> Self {
        id as i16
    }
}

#[derive(Debug, Default)]
pub struct ItemStack {
    pub item_id: ItemId,
    pub count: u8,
    pub damage: i16,
    // pub nbt: Option<Nbt>,
}

pub type Slot = ItemStack;

impl ItemStack {
    pub fn empty() -> Self {
        Self {
            item_id: ItemId::Air,
            count: 0,
            damage: 0,
            // nbt: None,
        }
    }

    pub fn new(item_id: ItemId, count: u8, damage: i16) -> Self {
        Self {
            item_id,
            count,
            damage,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.item_id == ItemId::Empty
    }
}

impl Encode for ItemStack {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let item_id: i16 = self.item_id.into();
        writer.write_all(&item_id.to_be_bytes())?;
        if self.is_empty() {
            return Ok(());
        }

        writer.write_all(&[self.count])?;
        writer.write_all(&self.damage.to_be_bytes())?;

        Ok(())
    }
}

impl Decode for ItemStack {
    fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
        todo!()
    }
}
