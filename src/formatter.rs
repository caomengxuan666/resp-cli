use colored::Colorize;
use redis::Value;

pub fn format_value(value: &Value) -> String {
    match value {
        Value::Array(values) => format_bulk(values),
        Value::BulkString(data) => format_data(data),
        Value::Int(i) => format_int(*i),
        Value::Nil => format_nil(),
        Value::SimpleString(s) => format_status(s),
        Value::Okay => "OK".green().bold().to_string(),
        _ => format!("{:?}", value).yellow().to_string(),
    }
}

fn format_bulk(values: &[Value]) -> String {
    if values.is_empty() {
        return "(empty list or set)".dimmed().to_string();
    }

    // For bulks, use redis-cli style numbered format
    let mut result = String::new();
    for (i, value) in values.iter().enumerate() {
        let item_num = i + 1;
        let formatted = format_value(value);

        // For nested bulks, add proper indentation
        let lines: Vec<&str> = formatted.lines().collect();
        for (j, line) in lines.iter().enumerate() {
            if j == 0 {
                result.push_str(&format!("{}> {}\n", item_num, line));
            } else {
                result.push_str(&format!("   {}\n", line));
            }
        }
    }
    result
}

fn format_data(data: &[u8]) -> String {
    match String::from_utf8(data.to_vec()) {
        Ok(s) => format_string(&s),
        Err(_) => format!("{:?}", data).yellow().to_string(),
    }
}

fn format_int(i: i64) -> String {
    i.to_string().green().bold().to_string()
}

fn format_nil() -> String {
    "(nil)".dimmed().italic().to_string()
}

fn format_status(s: &str) -> String {
    s.green().bold().to_string()
}

fn format_string(s: &str) -> String {
    if s.contains('\n') {
        // For multi-line strings, show them as raw text without quotes
        s.blue().italic().to_string()
    } else {
        format!("\"{}\"", s).blue().bold().to_string()
    }
}
