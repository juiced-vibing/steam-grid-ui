//! Minimal parser for Valve's text `KeyValues` format (.vdf / .acf).

#[derive(Debug, Clone)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

impl Vdf {
    /// Case-insensitive child lookup (Steam is inconsistent about key casing).
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Obj(v) => v.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v),
            Vdf::Str(_) => None,
        }
    }

    pub fn path(&self, keys: &[&str]) -> Option<&Vdf> {
        keys.iter().try_fold(self, |node, k| node.get(k))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Obj(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Obj(v) => v,
            Vdf::Str(_) => &[],
        }
    }
}

enum Tok {
    Open,
    Close,
    Text(String),
}

fn tokenize(text: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            c if c.is_whitespace() => {}
            '{' => out.push(Tok::Open),
            '}' => out.push(Tok::Close),
            '/' if it.peek() == Some(&'/') => {
                for c in it.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                let mut s = String::new();
                while let Some(c) = it.next() {
                    match c {
                        '"' => break,
                        '\\' => match it.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => s.push(other),
                            None => break,
                        },
                        _ => s.push(c),
                    }
                }
                out.push(Tok::Text(s));
            }
            _ => {
                let mut s = String::from(c);
                while let Some(&n) = it.peek() {
                    if n.is_whitespace() || n == '{' || n == '}' || n == '"' {
                        break;
                    }
                    s.push(n);
                    it.next();
                }
                // Platform conditionals like [$WIN32] carry no data we need.
                if !s.starts_with('[') {
                    out.push(Tok::Text(s));
                }
            }
        }
    }
    out
}

fn parse_obj(toks: &[Tok], pos: &mut usize, top: bool) -> Result<Vdf, String> {
    let mut items = Vec::new();
    loop {
        let key = match toks.get(*pos) {
            None if top => break,
            None => return Err("unexpected end of file inside block".into()),
            Some(Tok::Close) if !top => {
                *pos += 1;
                break;
            }
            Some(Tok::Text(k)) => k.clone(),
            Some(_) => return Err("unexpected brace where a key was expected".into()),
        };
        *pos += 1;
        let value = match toks.get(*pos) {
            Some(Tok::Text(v)) => {
                *pos += 1;
                Vdf::Str(v.clone())
            }
            Some(Tok::Open) => {
                *pos += 1;
                parse_obj(toks, pos, false)?
            }
            _ => return Err(format!("key \"{key}\" has no value")),
        };
        items.push((key, value));
    }
    Ok(Vdf::Obj(items))
}

pub fn parse(text: &str) -> Result<Vdf, String> {
    let toks = tokenize(text);
    parse_obj(&toks, &mut 0, true)
}

pub fn parse_file(path: &std::path::Path) -> Result<Vdf, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&String::from_utf8_lossy(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_blocks_and_escapes() {
        let v = parse(
            r#"
            // comment
            "libraryfolders"
            {
                "0" { "path" "C:\\Program Files (x86)\\Steam" "apps" { "228980" "0" } }
                "1" { "path" "G:\\SteamLibrary" }
            }"#,
        )
        .unwrap();
        let libs = v.get("LibraryFolders").unwrap();
        assert_eq!(libs.entries().len(), 2);
        assert_eq!(
            libs.path(&["0", "path"]).unwrap().as_str(),
            Some(r"C:\Program Files (x86)\Steam")
        );
        assert_eq!(libs.path(&["0", "apps", "228980"]).unwrap().as_str(), Some("0"));
    }

    #[test]
    fn rejects_unterminated_block() {
        assert!(parse(r#""a" { "b" "c""#).is_err());
    }
}
