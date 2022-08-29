use std::collections::BTreeMap;

#[derive(Debug)]
pub enum Value {
    Object(BTreeMap<String, Value>),
    Array(Vec<Value>),
    String(String),
    Number(f64),
    True,
    False,
    Null,
}

struct Context<'a> {
    idx: usize,
    line: usize,
    col: usize,
    src: &'a [u8],
}

impl<'a> Context<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            idx: 0,
            line: 1,
            col: 1,
            src: src.as_bytes(),
        }
    }

    fn peek(&self) -> Result<char, String> {
        if self.idx >= self.src.len() {
            return Err(self.error("unexpected EOF"));
        }

        let c = self.src[self.idx];
        Ok(c as char)
    }

    fn next(&mut self) -> Result<char, String> {
        let c = self.peek()?;
        self.consume()?;
        Ok(c)
    }

    fn consume(&mut self) -> Result<(), String> {
        let c = self.peek()?;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.idx += 1;
        Ok(())
    }

    fn is_next(&self) -> bool {
        self.idx < self.src.len()
    }

    fn error(&self, err: &str) -> String {
        format!("{} at {}:{}", err, self.line, self.col)
    }
}

fn parse_whitespace(ctx: &mut Context) -> Result<(), String> {
    while ctx.is_next() {
        match ctx.peek()? as u8 {
            0x20 | 0x0a | 0x0d | 0x09 => ctx.consume()?,
            _ => return Ok(()),
        }
    }
    Ok(())
}

fn parse_char(ctx: &mut Context, expected: char) -> Result<(), String> {
    let n = ctx.next()?;
    if n == expected {
        return Ok(());
    }
    Err(ctx.error(&format!("expected '{}'", expected)))
}

fn parse_word(ctx: &mut Context, expected: &str) -> Result<(), String> {
    for c in expected.chars() {
        parse_char(ctx, c)?;
    }
    Ok(())
}

fn parse_string(ctx: &mut Context) -> Result<Value, String> {
    parse_char(ctx, '"')?;
    let mut s = String::new();
    loop {
        match ctx.next()? {
            '"' => return Ok(Value::String(s)),
            '\\' => match ctx.next()? {
                '"' => s.push('"'),
                '\\' => s.push('\\'),
                '/' => s.push('/'),
                'b' => s.push(0x08 as char),
                'f' => s.push(0x0c as char),
                'n' => s.push(0x0a as char),
                'r' => s.push(0x0d as char),
                't' => s.push(0x09 as char),
                'u' => {
                    let mut n = 0_u16;
                    for i in 0..4 {
                        let d = ctx.next()? as u16;
                        if 48 <= d && d <= 57 {
                            // 0..9
                            n += (d - 48) * 16_u16.pow(4 - i - 1);
                        } else if 65 <= d && d <= 70 {
                            // A..F
                            n += (d - 55) * 16_u16.pow(4 - i - 1);
                        } else if 97 <= d && d <= 102 {
                            // a..f
                            n += (d - 87) * 16_u16.pow(4 - i - 1);
                        } else {
                            return Err(ctx.error("invalid hex digit"));
                        }
                    }
                    s.push(match char::from_u32(n as u32) {
                        Some(c) => c,
                        None => return Err(ctx.error("invalid character")),
                    });
                }
                _ => return Err(ctx.error("invalid character escape")),
            },
            c => s.push(c),
        }
    }
}

fn parse_digits(ctx: &mut Context) -> Result<f64, String> {
    let mut num_str = String::new();
    while ('0'..='9').contains(&ctx.peek()?) {
        let c = ctx.next()?;
        num_str.push(c);
    }
    let mut num = 0_f64;
    for i in 0..num_str.len() {
        let c = (num_str.chars().nth(i).unwrap() as u8) as f64;
        num += (10_f64).powf((num_str.len() - i - 1) as f64) * (c - 48_f64);
    }
    Ok(num)
}

fn parse_number(ctx: &mut Context) -> Result<Value, String> {
    let mut num;
    match ctx.peek()? {
        '-' => {
            ctx.consume()?;
            num = -1_f64 * parse_digits(ctx)?;
        }
        '0'..='9' => num = parse_digits(ctx)?,
        _ => return Err(ctx.error("expected '-' or '0'..'9'")),
    }

    if ctx.peek()? == '.' {
        ctx.consume()?;
        let mut fraction = parse_digits(ctx)?;
        while fraction > 1_f64 {
            fraction /= 10_f64;
        }
        num += fraction;
    }

    if matches!(ctx.peek()?, 'e' | 'E') {
        ctx.consume()?;
        let sign = match ctx.peek()? {
            '+' => {
                ctx.consume()?;
                1_f64
            }
            '-' => {
                ctx.consume()?;
                -1_f64
            }
            _ => 1_f64,
        };

        let exp = sign * parse_digits(ctx)?;
        num *= 10_f64.powf(exp);
    }

    Ok(Value::Number(num))
}

fn parse_intrisic(ctx: &mut Context) -> Result<Value, String> {
    match ctx.peek()? {
        't' => {
            parse_word(ctx, "true")?;
            Ok(Value::True)
        }
        'f' => {
            parse_word(ctx, "false")?;
            Ok(Value::False)
        }
        'n' => {
            parse_word(ctx, "null")?;
            Ok(Value::Null)
        }
        _ => Err(ctx.error("expected 'true', 'false', or 'null'")),
    }
}

fn parse_array(ctx: &mut Context) -> Result<Value, String> {
    parse_char(ctx, '[')?;
    if ctx.peek()? == ']' {
        ctx.consume()?;
        return Ok(Value::Array(Vec::new()));
    }
    let mut vals = Vec::new();

    loop {
        parse_whitespace(ctx)?;

        vals.push(parse_value(ctx)?);

        parse_whitespace(ctx)?;
        match ctx.next()? {
            ']' => break,
            ',' => (),
            _ => return Err(ctx.error("expected ']' or ','")),
        }
    }
    Ok(Value::Array(vals))
}

fn parse_object(ctx: &mut Context) -> Result<Value, String> {
    parse_char(ctx, '{')?;
    parse_whitespace(ctx)?;

    let mut obj_vals = BTreeMap::new();
    'outer: loop {
        match ctx.peek()? {
            '"' => loop {
                let key = match parse_string(ctx)? {
                    Value::String(s) => s,
                    _ => unreachable!(),
                };

                parse_whitespace(ctx)?;
                parse_char(ctx, ':')?;

                let val = parse_value(ctx)?;
                obj_vals.insert(key, val);

                parse_whitespace(ctx)?;
                match ctx.next()? {
                    '}' => break 'outer,
                    ',' => {
                        parse_whitespace(ctx)?;
                    }
                    _ => return Err(ctx.error("expected '}' or ','")),
                }
            },
            '}' => {
                ctx.consume()?;
                break;
            }
            _ => return Err(ctx.error("expected '\"' or '}'")),
        }
    }
    Ok(Value::Object(obj_vals))
}

fn parse_value(ctx: &mut Context) -> Result<Value, String> {
    parse_whitespace(ctx)?;
    match ctx.peek()? {
        '{' => parse_object(ctx),
        '"' => parse_string(ctx),
        '[' => parse_array(ctx),
        '-' | '0'..='9' => parse_number(ctx),
        _ => parse_intrisic(ctx),
    }
}

pub fn parse(src: &str) -> Result<Value, String> {
    let mut ctx = Context::new(src);
    parse_value(&mut ctx)
}
