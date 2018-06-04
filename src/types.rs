#![allow(non_snake_case)]
use errors::{BerylliumError, BerylliumResult};
use futures::Future;
use hyper::Client;
use hyper::header::ContentType;
use hyper_rustls::HttpsConnector;
use image::{self, GenericImage, ImageFormat as ImgFormat};
use mime::{IMAGE_BMP, IMAGE_GIF};
use serde::de::{self, Visitor, SeqAccess, MapAccess, Deserialize, Deserializer, Error as DecodeError};
use serde_json::{Value, from_str};
use serde_json::error::Error as SerdeError;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::Path;
use std::fmt::{self};
use uuid::Uuid;

// FIXME: Check the types (for example, id should be Uuid instead of String),

/// HTTPS client (courtesy of rustls)
pub type HyperClient = Client<HttpsConnector>;
/// The `Future` type used throughout the lib.
pub type BerylliumFuture<I> = Box<Future<Item=I, Error=BerylliumError>>;
/// A closure which takes a HTTPS client and returns a `Future`. This is
/// how HTTPS client requests are queued in the event loop.
pub type EventLoopRequest<I> = Box<Fn(&HyperClient) -> BerylliumFuture<I> + Send + 'static>;

/// Represents the type of event handed over to the user.
pub enum Event {
    ConversationMemberJoin {
        members_joined: Vec<Uuid>,
    },
    ConversationMemberLeave {
        members_left: Vec<Uuid>,
    },
    ConversationRename,
    Message {
        text: String,
        from: String,       // FIXME: Should be `Uuid`
    },
    Image,
}

/// Event data passed to the type implementing the `Handler` trait.
pub struct EventData {
    /// ID of this bot instance.
    pub bot_id: Uuid,
    /// Conversation data
    pub conversation: Conversation,
    /// Event-type and related data (if any)
    pub event: Event,
}

/// Represents a conversation member.
#[derive(Clone, Deserialize, Serialize)]
pub struct Member {
    pub id: Uuid,
    pub status: i8,
}

// Custom implementations for HashSet addressing.
// We don't care about anything other than the member's UUID.
impl Borrow<Uuid> for Member {
    fn borrow(&self) -> &Uuid {
        &self.id
    }
}

impl Hash for Member {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Member {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Member {}

#[derive(Clone, Deserialize, Serialize)]
pub struct Conversation {
    pub id: Uuid,
    pub name: String,
    pub members: HashSet<Member>,
}

#[derive(Deserialize, Serialize)]
pub struct Origin {
    pub id: Uuid,
    pub name: String,
    pub handle: String,
    pub accent_id: i8,
}

#[derive(Deserialize, Serialize)]
pub struct BotCreationData {
    pub id: Uuid,
    pub client: String,
    pub origin: Origin,
    pub conversation: Conversation,
    pub token: String,
    pub locale: String,
}

#[derive(Default, Deserialize)]
pub struct Devices {
    // UserID -> [ClientID]
    pub missing: HashMap<String, Vec<String>>,
}

#[derive(Clone, Copy, Debug)]
pub enum ConversationEventType {
    MessageAdd,
    MemberJoin,
    MemberLeave,
    Rename,
}

fn deserialize_conv_event_type<'de, D>(de: D) -> Result<ConversationEventType, D::Error>
    where D: Deserializer<'de>
{
    let deser_result: Value = Deserialize::deserialize(de)?;
    match deser_result {
        Value::String(ref s) if s == "conversation.otr-message-add"
            => Ok(ConversationEventType::MessageAdd),
        Value::String(ref s) if s == "conversation.member-join"
            => Ok(ConversationEventType::MemberJoin),
        Value::String(ref s) if s == "conversation.member-leave"
            => Ok(ConversationEventType::MemberLeave),
        Value::String(ref s) if s == "conversation.rename"
            => Ok(ConversationEventType::Rename),
        _ => Err(DecodeError::custom("Unexpected value for ConversationEventType")),
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ConversationData {
    MessageAdd {
        sender: String,
        recipient: String,
        text: String,
    },
    LeavingOrJoiningMembers {
        user_ids: Vec<Uuid>,
    },
    Rename {
        name: String,
    }
}

#[derive(Deserialize)]
pub struct MessageData {
    #[serde(rename = "type")]
    #[serde(deserialize_with = "deserialize_conv_event_type")]
    pub type_: ConversationEventType,
    pub conversation: String,
    pub from: String,
    pub data: ConversationData,
    pub time: String,
}

#[derive(Deserialize, Serialize)]
pub struct EncodedPreKey {
    pub id: u16,
    pub key: String,
}

pub type DevicePreKeys = HashMap<String, HashMap<String, EncodedPreKey>>;

#[derive(Serialize)]
pub struct BotCreationResponse {
    pub prekeys: Vec<EncodedPreKey>,
    pub last_prekey: EncodedPreKey,
}

#[derive(Serialize)]
pub struct MessageRequest<'a, 'b> {
    pub sender: &'a str,
    pub recipients: HashMap<&'b str, HashMap<&'b str, String>>,
}

#[derive(Serialize)]
pub struct AssetUploadRequest<'a> {
    pub public: bool,
    pub retention: &'a str,
}

#[derive(Deserialize)]
pub struct AssetData {
    pub key: String,
    pub token: String,
}

// cex types
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ResType {
    VcoinTickerRes {
        timestamp: String,
        low: String,
        high: String,
        last: String,
        volume: String,
        volume30d: String,
        bid: f64,
        ask: f64,
    },
    Vchart(Vec<Vcoinchartpoint>),
    Vcandle(CandleData0),

}



#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum ReqType {
    VcoinTickerReq {
        lastHours: u32,
        maxRespArrSize: u32,
    }


}



#[derive(Debug, Deserialize)]
pub struct VcoinCandleRes {
    pub timestamp: String,
    pub open:f64,
    pub low: String,
    pub high: String,
    pub last: String,
    pub volume: String,
    pub volume30d: String,
    pub bid: f64,
    pub ask: f64,
}
//
//
//
#[derive(Debug, Deserialize)]
pub struct Vcoinchartpoint{
    pub tmsp: u64,
    pub price: String,
}



#[derive(Debug)]
pub struct CandleData0 {
    time: u32,
    data1m:Vec<CandleData>,
    data1h:Vec<CandleData>,
    data1d:Vec<CandleData>,
}



impl CandleData0 {
    fn new(t: u32, m: &str, h: &str, d: &str) -> Result<Self, SerdeError> {
        Ok(CandleData0{
            time:t
            , data1m: from_str(m)?
            , data1h: from_str(h)?
            , data1d: from_str(d)?
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CandleData {
    ts: u32,
    o: f64,
    l: f64,
    h: f64,
    c: f64,
    v: f64
}



impl<'de> Deserialize<'de> for CandleData0 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        enum Field { Time, Data1m, Data1h, Data1d };

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
                where
                    D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`Time` or `Data1m` or `Data1h` or `Data1d`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                        where
                            E: de::Error,
                    {
                        match value {
                            "time" => Ok(Field::Time),
                            "data1m" => Ok(Field::Data1m),
                            "data1h" => Ok(Field::Data1h),
                            "data1d" => Ok(Field::Data1d),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct CandleData0Visitor;

        impl<'de> Visitor<'de> for CandleData0Visitor {
            type Value = CandleData0;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Duration")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<CandleData0, V::Error>
                where
                    V: SeqAccess<'de>,
            {
                let time = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let data1m = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let data1h = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let data1d = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;


                CandleData0::new(time, data1m, data1h, data1d).map_err(de::Error::custom)
            }

            fn visit_map<V>(self, mut map: V) -> Result<CandleData0, V::Error>
                where
                    V: MapAccess<'de>,
            {
                let mut time = None;
                let mut data1m = None;
                let mut data1h = None;
                let mut data1d = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Time => {
                            if time.is_some() {
                                return Err(de::Error::duplicate_field("time"));
                            }
                            time = Some(map.next_value()?);
                        }
                        Field::Data1m => {
                            if data1m.is_some() {
                                return Err(de::Error::duplicate_field("data1m"));
                            }
                            data1m = Some(map.next_value()?);
                        }
                        Field::Data1h => {
                            if data1h.is_some() {
                                return Err(de::Error::duplicate_field("data1h"));
                            }
                            data1h = Some(map.next_value()?);
                        }
                        Field::Data1d => {
                            if data1d.is_some() {
                                return Err(de::Error::duplicate_field("data1d"));
                            }
                            data1d = Some(map.next_value()?);
                        }
                    }
                }
                let time = time.ok_or_else(|| de::Error::missing_field("time"))?;
                let data1m = data1m.ok_or_else(|| de::Error::missing_field("data1m"))?;
                let data1h = data1h.ok_or_else(|| de::Error::missing_field("data1h"))?;
                let data1d = data1d.ok_or_else(|| de::Error::missing_field("data1d"))?;
                CandleData0::new(time, data1m, data1h, data1d).map_err(de::Error::custom)
            }
        }

        const FIELDS: &'static [&'static str] = &["time", "data1m", "data1h","data1d"];
        deserializer.deserialize_struct("CandleData0", FIELDS, CandleData0Visitor)
    }
}

//cex  types end

pub enum MessageStatus {
    Sent,
    Failed(Devices),
}

pub struct EncryptData {
    pub key: Vec<u8>,
    pub data: Vec<u8>,
    pub hash: Vec<u8>,
}

#[derive(Clone)]
/// Represents an image with a known format.
pub struct Image {
    meta: ImageMeta,
    data: Vec<u8>,
}

#[derive(Clone, Copy)]
/// Metadata required for an image to upload in Wire.
pub struct ImageMeta {
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
}

impl Image {
    /// Open an image from the given path. This opens the file
    /// and passes it to `Image::from_reader`
    pub fn from_path<P>(path: P) -> BerylliumResult<Self>
        where P: AsRef<Path>
    {
        let fd = File::open(path)?;
        Self::from_reader(fd)
    }

    /// Open an image with the given reader. Note that this just
    /// reads all bytes and passes it to `Image::from_bytes`
    pub fn from_reader<R>(mut reader: R) -> BerylliumResult<Self>
        where R: Read
    {
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes)?;
        Self::from_bytes(bytes)
    }

    /// Gets the metadata (image format, width and height) from the data,
    /// (uses [this function](https://docs.rs/image/*/image/fn.guess_format.html)
    /// internally) to obtain the format.
    pub fn from_bytes(bytes: Vec<u8>) -> BerylliumResult<Self> {
        let fmt = match image::guess_format(&bytes)? {
            ImgFormat::JPEG => ImageFormat::Jpeg,
            ImgFormat::PNG  => ImageFormat::Png,
            ImgFormat::GIF  => ImageFormat::Gif,
            ImgFormat::BMP  => ImageFormat::Bmp,
            _ => return Err(BerylliumError::Other(String::from("Unsupported image format")))
        };

        let img = image::load_from_memory(&bytes)?;
        Ok(Image {
            meta: ImageMeta {
                format: fmt,
                width: img.width(),
                height: img.height(),
            },
            data: bytes,
        })
    }

    pub fn metadata(&self) -> ImageMeta {
        self.meta
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

// NOTE: If you want more, feel free to open a PR! :)
#[derive(Clone, Copy)]
pub enum ImageFormat {
    Bmp,
    Gif,
    Jpeg,
    Png,
}

impl ImageFormat {
    pub fn mime(&self) -> String {
        let s = match *self {
            ImageFormat::Bmp  => "image/bmp",
            ImageFormat::Gif  => "image/gif",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png  => "image/png",
        };

        s.to_owned()
    }
}

impl Into<ContentType> for ImageFormat {
    fn into(self) -> ContentType {
        match self {
            ImageFormat::Bmp  => ContentType(IMAGE_BMP),
            ImageFormat::Gif  => ContentType(IMAGE_GIF),
            ImageFormat::Jpeg => ContentType::jpeg(),
            ImageFormat::Png  => ContentType::png(),
        }
    }
}
