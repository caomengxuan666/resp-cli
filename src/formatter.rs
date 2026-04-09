use colored::Colorize;
use redis::Value;

pub fn format_value(value: &Value) -> String {
    match value {
        Value::Bulk(values) => format_bulk(values),
        Value::Data(data) => format_data(data),
        Value::Int(i) => format_int(*i),
        Value::Nil => format_nil(),
        Value::Status(s) => format_status(s),
        Value::Okay => "OK".green().bold().to_string(),
    }
}

fn format_bulk(values: &[Value]) -> String {
    if values.is_empty() {
        return "(empty list or set)".dimmed().to_string();
    }
    
    let mut result = String::new();
    result.push_str(&format!("{}
", "[".cyan()));
    
    for (i, value) in values.iter().enumerate() {
        let formatted = format_value(value);
        let line = if i == values.len() - 1 {
            format!("    {}", formatted)
        } else {
            format!("    {},", formatted)
        };
        result.push_str(&format!("{}\n", line));
    }
    
    result.push_str(&"]".cyan());
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
        format!("{:?}", s).blue().italic().to_string()
    } else {
        format!("\"{}\"", s).blue().bold().to_string()
    }
}
