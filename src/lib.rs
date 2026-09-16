use std::collections::BTreeMap;

const REMOVED_COLUMNS: [&str; 8] = [
    "Value Date",
    "Partner Iban",
    "Type",
    "Payment Reference",
    "Account Name",
    "Original Amount",
    "Original Currency",
    "Exchange Rate",
];

#[derive(Debug, Clone)]
struct Transaction {
    partner: String,
    amount: f64,
}

fn parse_csv(input: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = input.chars().peekable();

    while let Some(character) = chars.next() {
        match character {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                row.push(field.trim_end_matches('\r').to_owned());
                field.clear();
            }
            '\n' if !quoted => {
                row.push(field.trim_end_matches('\r').to_owned());
                if row.iter().any(|value| !value.is_empty()) {
                    rows.push(row);
                }
                row = Vec::new();
                field.clear();
            }
            _ => field.push(character),
        }
    }

    if quoted {
        return Err("A CSV file contains an unterminated quoted field.".to_owned());
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field.trim_end_matches('\r').to_owned());
        if row.iter().any(|value| !value.is_empty()) {
            rows.push(row);
        }
    }
    if rows.is_empty() {
        return Err("A selected CSV file is empty.".to_owned());
    }
    Ok(rows)
}

fn parse_payload(payload: &str) -> Result<(Vec<(String, String)>, Vec<String>), String> {
    let mut cursor = 0;
    let mapping_length = read_length(payload, &mut cursor, "mapping")?;
    let mapping_end = cursor + mapping_length;
    if mapping_end > payload.len() {
        return Err("The mapping payload is truncated.".to_owned());
    }
    let mapping_text = &payload[cursor..mapping_end];
    cursor = mapping_end;
    let mut mapping = Vec::new();
    for line in mapping_text.lines() {
        let mut parts = line.splitn(2, '\t');
        let category = parts.next().unwrap_or_default();
        let item = parts.next().unwrap_or_default();
        if !category.is_empty() && !item.is_empty() {
            mapping.push((category.to_owned(), item.to_owned()));
        }
    }

    let file_count = read_length(payload, &mut cursor, "file count")?;
    let mut files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        let name_length = read_length(payload, &mut cursor, "file name")?;
        cursor += name_length;
        if cursor > payload.len() {
            return Err("The file name payload is truncated.".to_owned());
        }
        let csv_length = read_length(payload, &mut cursor, "CSV")?;
        let end = cursor + csv_length;
        if end > payload.len() {
            return Err("A CSV payload is truncated.".to_owned());
        }
        files.push(payload[cursor..end].to_owned());
        cursor = end;
    }
    if files.is_empty() {
        return Err("Choose at least one statement CSV file.".to_owned());
    }
    Ok((mapping, files))
}

fn read_length(payload: &str, cursor: &mut usize, label: &str) -> Result<usize, String> {
    let remainder = &payload[*cursor..];
    let newline = remainder
        .find('\n')
        .ok_or_else(|| format!("The {label} payload is malformed."))?;
    let value = remainder[..newline]
        .parse::<usize>()
        .map_err(|_| format!("The {label} length is invalid."))?;
    *cursor += newline + 1;
    Ok(value)
}

fn calculate(mapping: Vec<(String, String)>, files: Vec<String>) -> Result<String, String> {
    let mut transactions = Vec::new();
    let mut total_rows = 0;
    let file_count = files.len();
    for file in files {
        let rows = parse_csv(&file)?;
        let headers = &rows[0];
        let partner_index = headers
            .iter()
            .position(|header| header == "Partner Name")
            .ok_or_else(|| "Column 'Partner Name' not found in a statement CSV.".to_owned())?;
        let amount_index = headers
            .iter()
            .position(|header| header == "Amount (EUR)")
            .ok_or_else(|| "Column 'Amount (EUR)' not found in a statement CSV.".to_owned())?;
        let _ = &REMOVED_COLUMNS;
        for row in rows.iter().skip(1) {
            total_rows += 1;
            let amount_text = row.get(amount_index).map(String::as_str).unwrap_or_default();
            let amount = amount_text
                .parse::<f64>()
                .map_err(|_| "Column 'Amount (EUR)' contains a non-numeric value.".to_owned())?;
            if amount < 0.0 {
                transactions.push(Transaction {
                    partner: row.get(partner_index).cloned().unwrap_or_default(),
                    amount: amount.abs(),
                });
            }
        }
    }

    let mut summary = BTreeMap::<String, f64>::new();
    for transaction in &transactions {
        let category = mapping
            .iter()
            .find(|(_, item)| transaction.partner.contains(item))
            .map(|(category, _)| category.as_str())
            .unwrap_or("NO CATEGORY");
        *summary.entry(category.to_owned()).or_insert(0.0) += transaction.amount;
    }
    let mut summary: Vec<(String, f64)> = summary.into_iter().collect();
    summary.sort_by(|left, right| right.1.total_cmp(&left.1));
    let grand_total: f64 = summary.iter().map(|(_, amount)| amount).sum();

    let rows_json = summary
        .iter()
        .map(|(category, amount)| {
            format!(
                "{{\"category\":\"{}\",\"amount\":{:.2}}}",
                escape_json(category),
                amount
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "{{\"files\":{},\"totalRows\":{},\"outgoingRows\":{},\"grandTotal\":{:.2},\"categories\":[{}]}}",
        file_count, total_rows, transactions.len(), grand_total, rows_json
    ))
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[no_mangle]
pub extern "C" fn alloc(length: usize) -> *mut u8 {
    let mut buffer = Vec::with_capacity(length);
    let pointer = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    pointer
}

static mut RESULT_POINTER: *mut u8 = std::ptr::null_mut();
static mut RESULT_LENGTH: usize = 0;

#[no_mangle]
pub unsafe extern "C" fn process_pipeline(pointer: *const u8, length: usize) -> u32 {
    let bytes = std::slice::from_raw_parts(pointer, length);
    let payload = String::from_utf8_lossy(bytes);
    let result = match parse_payload(&payload).and_then(|(mapping, files)| calculate(mapping, files)) {
        Ok(value) => format!("{{\"ok\":true,\"data\":{}}}", value),
        Err(error) => format!("{{\"ok\":false,\"error\":\"{}\"}}", escape_json(&error)),
    };
    let mut bytes = result.into_bytes();
    let pointer = bytes.as_mut_ptr();
    let length = bytes.len();
    std::mem::forget(bytes);
    RESULT_POINTER = pointer;
    RESULT_LENGTH = length;
    pointer as u32
}

#[no_mangle]
pub unsafe extern "C" fn result_pointer() -> u32 {
    RESULT_POINTER as u32
}

#[no_mangle]
pub unsafe extern "C" fn result_length() -> u32 {
    RESULT_LENGTH as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_csv_fields() {
        let rows = parse_csv("Partner Name,Amount (EUR)\n\"KFC, Center\",-12.5\n").unwrap();
        assert_eq!(rows[1][0], "KFC, Center");
    }

    #[test]
    fn filters_and_summarizes_outgoing_transactions() {
        let result = calculate(
            vec![("Food".to_owned(), "KFC".to_owned())],
            vec!["Partner Name,Amount (EUR)\nKFC,-12\nSalary,2000\n".to_owned()],
        )
        .unwrap();
        assert!(result.contains("\"grandTotal\":12.00"));
        assert!(result.contains("\"category\":\"Food\""));
    }
}