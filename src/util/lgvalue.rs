use serde::{Serialize, Deserialize};
use futures::stream::Stream;
use mime::Mime;

#[derive(Serialize, Deserialize, Debug)]
pub struct LGValue {
    #[serde(flatten)]
    value:  serde_json::value::Value,
    #[serde(skip)]
    mimet:  Vec<mime::Mime>,
    eof:    bool,
}

pub struct LGValueError;

impl LGValue {
    fn get_bytes(&self) -> Result<Bytes, LGError> {
        let mut b = actix_web::web::BytesMut::new();
        for n in self.mimet.iter() {
            match n.type_().as_str() {
                "json"      => serde_json::to_writer(&mut b, self)
                    .map_err(|_| LGValueError)?,
                "yaml" |
                "x-yaml"    => serde_yaml::to_writer(&mut b, self)
                    .map_err(|_| LGValueError)?,
                "xml"       => quick_xml::se::to_writer(&mut b, self)
                    .map_err(|_| LGValueError)?,
                "msgpack"   => rmp_serde::encode::write(&mut b, self)
                    .map_err(|_| LGValueError)?,
            }
        }
        b.into()
    }
}

impl Stream for LGValue {
    type Item = Result<Bytes, LGError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>> {
        if self.eof == false {
            unsafe { self.get_unchecked_mut() = true };
            poll::Ready(Some(self.get_bytes()))
        } else {
            poll::Ready(None)
        }
    }
}
