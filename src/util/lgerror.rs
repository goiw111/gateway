pub struct LGError {
    inner:  box<dyn LGEInfo>,
    eof:    bool,
    to:     Mime,
}

impl LGError {

}

trait LGEInfo {
    fn status_code(&self) -> u16;
    fn message_text(&self) -> &str;
    fn description_text(&self) -> &str;
    fn debug_description_test(&self) -> &str;
}

impl actix_web::ResponseError for LGError {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::from_u16(self.inner.status_code())
    }
    fn error_response(&self) -> web::HttpResponse {
        web::HttpResponse::build(self.status_code())
            .
    }
}

impl Stream for LGError {
    type Item = Result<Bytes, LGError>
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>> {
        if self.eof == false {
            unsafe { self.get_unchecked_mut() = true }
            let mut b = actix_web::web::BytesMut::new();
            match self.to.type_().as_str() {
                "json"
                    => serde_json::to_writer(&mut b, self),
                "xml"
                    => quick_xml::se::to_writer(&mut b, self),
                "yaml" | "x-yaml"
                    => serde_yaml::to_writer(&mut b, self),
                "msgpack"
                    => rmp_serde::encode::write(&mut b, self),
                _   => serde_json::to_writer(&mut b, self),
            }
            Poll::Ready(Some(Ok(b.into())))
        } else {
            Poll::Ready(None)
        }
    }
}

impl Serialize for LGError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sv = serializer.serialize_struct("error", 3)?;
        sv.serialize_field("code", &self.inner.status_code().to_string())?;
        sv.serialize_field("message", self.inner.message_text())?;
        sv.serialize_field("description", self.inner.description_text())?;
        sv.end()
    }
}


impl<E: LGEInfo> From<E> for LGError {
    fn from(error: E) -> self {
        LGError {
            inner:  Box::new(eroor);
        }
    }
}


