//! Minimal ordered JSON value, so the CLI-owned result keeps the documented key order without extra
//! dependencies. String escaping is delegated to `serde_json`.

pub enum Json {
    Null,
    Num(u64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(&'static str, Json)>),
}

impl Json {
    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    /// Compact rendering followed by exactly one newline.
    pub fn to_line(&self) -> String {
        let mut out = String::new();
        self.render(&mut out);
        out.push('\n');
        out
    }

    fn render(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Num(n) => out.push_str(&n.to_string()),
            Json::Str(s) => out.push_str(&serde_json::Value::String(s.clone()).to_string()),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.render(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&serde_json::Value::String((*k).to_owned()).to_string());
                    out.push(':');
                    v.render(out);
                }
                out.push('}');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_compact_single_newline() {
        let j = Json::Obj(vec![
            ("b", Json::Num(1)),
            ("a", Json::Arr(vec![Json::Null, Json::str("x\"\n")])),
            ("o", Json::Obj(vec![])),
        ]);
        assert_eq!(
            j.to_line(),
            "{\"b\":1,\"a\":[null,\"x\\\"\\n\"],\"o\":{}}\n"
        );
    }
}
