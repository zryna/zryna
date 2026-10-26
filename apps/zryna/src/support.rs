//! Static public M4 contract rendering; no execution or grant authority is constructed.

use std::{
    io::{self, Write},
    process::ExitCode,
};

const CONTRACT: &str = include_str!("m4_support.json");

#[derive(Clone, Debug, clap::Args)]
pub(super) struct Options {
    /// Emit the compiled canonical support contract JSON.
    #[arg(long)]
    json: bool,
}

pub(super) fn report(options: &Options) -> ExitCode {
    let mut output = io::stdout().lock();
    let result = if options.json {
        output.write_all(&canonical_contract(CONTRACT))
    } else {
        output.write_all(
            b"M4 static support contract; execution and publication are not authenticated.\n\
Checkout component/browser, checkout or fresh verified-installation command, and checkout \
status/clock server contracts have bounded host rows.\n\
Installed component/browser/server and existing immutable beta H1 remain excluded.\n\
Use zryna support --json for the complete matrix and registry digest.\n",
        )
    };
    if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

// The generated embedding is trusted compact JSON. Preserve every token and its order;
// restore only the established two-space public layout, without value reserialization.
fn canonical_contract(contract: &str) -> Vec<u8> {
    fn newline(output: &mut Vec<u8>, depth: usize) {
        output.push(b'\n');
        output.resize(output.len() + depth * 2, b' ');
    }
    let bytes = contract.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let (mut quoted, mut escaped) = (false, false);
    let mut depth = 0usize;
    let mut previous = 0u8;
    for (index, &byte) in bytes.iter().enumerate() {
        if quoted {
            output.push(byte);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            continue;
        }
        if byte.is_ascii_whitespace() {
            continue;
        }
        match byte {
            b'"' => {
                quoted = true;
                output.push(byte);
            }
            b'{' | b'[' => {
                output.push(byte);
                depth += 1;
                let close = if byte == b'{' { b'}' } else { b']' };
                if bytes[index + 1..]
                    .iter()
                    .find(|next| !next.is_ascii_whitespace())
                    != Some(&close)
                {
                    newline(&mut output, depth);
                }
            }
            b'}' | b']' => {
                depth = depth.checked_sub(1).expect("generated support nesting");
                if !matches!(previous, b'{' | b'[') {
                    newline(&mut output, depth);
                }
                output.push(byte);
            }
            b',' => {
                output.push(byte);
                newline(&mut output, depth);
            }
            b':' => output.extend_from_slice(b": "),
            _ => output.push(byte),
        }
        previous = byte;
    }
    output.push(b'\n');
    output
}

#[cfg(test)]
mod tests {
    use super::canonical_contract;

    #[test]
    fn canonical_layout_preserves_nested_containers_utf8_and_string_punctuation() {
        let compact = r#"{"z":[true,null,{"empty":[],"map":{}}],"text":"escaped \" quote, \\ slash, \n newline and 雪 : , [ ] { }"}"#;
        let expected = r#"{
  "z": [
    true,
    null,
    {
      "empty": [],
      "map": {}
    }
  ],
  "text": "escaped \" quote, \\ slash, \n newline and 雪 : , [ ] { }"
}
"#;
        assert_eq!(canonical_contract(compact), expected.as_bytes());
    }

    #[test]
    fn canonical_layout_keeps_token_spelling_key_order_and_one_final_newline() {
        for (compact, expected) in [
            ("{}\n", "{}\n"),
            (" [] \r\n", "[]\n"),
            (
                r#"{"z":-0,"a":1e+20,"b":1.25}"#,
                "{\n  \"z\": -0,\n  \"a\": 1e+20,\n  \"b\": 1.25\n}\n",
            ),
        ] {
            assert_eq!(canonical_contract(compact), expected.as_bytes());
        }
    }
}
